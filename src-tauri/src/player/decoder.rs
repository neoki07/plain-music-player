use super::Source;
use crate::player::source::{
    Amplify, FadeIn, Pausable, PeriodicAccess, SamplesConverter, Speed, Stoppable, TakeDuration,
};
use crate::player::Sample;
use std::cmp::max;
use std::{fmt, fs::File, time::Duration};
use symphonia::{
    core::{
        audio::{AudioBufferRef, SampleBuffer, SignalSpec},
        codecs::{self, CodecParameters},
        errors::Error,
        formats::{FormatOptions, FormatReader, SeekMode, SeekTo},
        io::{MediaSourceStream, MediaSourceStreamOptions},
        meta::MetadataOptions,
        probe::Hint,
        units::{Time, TimeBase},
    },
    default::get_probe,
};

// Decoder errors are not considered fatal.
// The correct action is to just get a new packet and try again.
// But a decode error in more than 3 consecutive packets is fatal.
const MAX_DECODE_ERRORS: usize = 3;

pub struct Symphonia {
    decoder: Box<dyn codecs::Decoder>,
    current_frame_offset: usize,
    format: Box<dyn FormatReader>,
    buffer: SampleBuffer<i16>,
    spec: SignalSpec,
    duration: Duration,
    elapsed: Duration,
    fade_in_remaining_ns: f32,
    fade_in_total_ns: f32,
    fade_out_remaining_ns: f32,
    fade_out_total_ns: f32,
    is_seeking_soon: bool,
    seek_to_time: Duration,
}

impl Symphonia {
    pub fn new(file: File, gapless: bool) -> Result<Self, SymphoniaDecoderError> {
        let source = Box::new(file);

        let mss = MediaSourceStream::new(source, MediaSourceStreamOptions::default());
        match Self::init(mss, gapless) {
            Err(e) => match e {
                Error::IoError(e) => Err(SymphoniaDecoderError::IoError(e.to_string())),
                Error::DecodeError(e) => Err(SymphoniaDecoderError::DecodeError(e)),
                Error::SeekError(_) => {
                    unreachable!("Seek errors should not occur during initialization")
                }
                Error::Unsupported(_) => Err(SymphoniaDecoderError::UnrecognizedFormat),
                Error::LimitError(e) => Err(SymphoniaDecoderError::LimitError(e)),
                Error::ResetRequired => Err(SymphoniaDecoderError::ResetRequired),
            },
            Ok(Some(decoder)) => Ok(decoder),
            Ok(None) => Err(SymphoniaDecoderError::NoStreams),
        }
    }

    fn init(
        mss: MediaSourceStream,
        gapless: bool,
    ) -> symphonia::core::errors::Result<Option<Self>> {
        let mut probed = get_probe().format(
            &Hint::default(),
            mss,
            &FormatOptions {
                prebuild_seek_index: true,
                seek_index_fill_rate: 10,
                enable_gapless: gapless,
                // enable_gapless: false,
            },
            &MetadataOptions::default(),
        )?;

        let track = match probed.format.default_track() {
            Some(stream) => stream,
            None => return Ok(None),
        };

        let mut decoder = symphonia::default::get_codecs().make(
            &track.codec_params,
            &codecs::DecoderOptions { verify: true },
        )?;

        let duration = Self::get_duration(&track.codec_params);

        let mut decode_errors: usize = 0;
        let decode_result = loop {
            let current_frame = probed.format.next_packet()?;
            match decoder.decode(&current_frame) {
                Ok(result) => break result,
                Err(e) => match e {
                    Error::DecodeError(_) => {
                        decode_errors += 1;
                        if decode_errors > MAX_DECODE_ERRORS {
                            return Err(e);
                        }
                    }
                    _ => return Err(e),
                },
            }
        };
        let spec = *decode_result.spec();
        let buffer = Self::get_buffer(decode_result, spec);

        Ok(Some(Self {
            decoder,
            current_frame_offset: 0,
            format: probed.format,
            buffer,
            spec,
            duration,
            elapsed: Duration::from_secs(0),
            fade_in_remaining_ns: 0.0,
            fade_in_total_ns: 0.0,
            fade_out_remaining_ns: 0.0,
            fade_out_total_ns: 0.0,
            is_seeking_soon: false,
            seek_to_time: Duration::from_secs(0),
        }))
    }

    fn get_duration(params: &CodecParameters) -> Duration {
        // if let Some(n_frames) = params.n_frames {
        //     if let Some(tb) = params.time_base {
        //         let time = tb.calc_time(n_frames);
        //         Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
        //     } else {
        //         panic!("no time base?");
        //     }
        // } else {
        //     panic!("no n_frames");
        // }

        params.n_frames.map_or_else(
            || {
                // panic!("no n_frames");
                Duration::from_secs(99)
            },
            |n_frames| {
                params.time_base.map_or_else(
                    || {
                        // panic!("no time base?");
                        Duration::from_secs(199)
                    },
                    |tb| {
                        let time = tb.calc_time(n_frames);
                        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac)
                    },
                )
            },
        )
    }

    #[inline]
    fn get_buffer(decoded: AudioBufferRef, spec: SignalSpec) -> SampleBuffer<i16> {
        let duration = decoded.capacity() as u64;
        let mut buffer = SampleBuffer::<i16>::new(duration * 10, spec);
        buffer.copy_interleaved_ref(decoded);
        buffer
    }
}

impl Source for Symphonia {
    #[inline]
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.buffer.samples().len())
    }

    #[inline]
    #[allow(clippy::cast_possible_truncation)]
    fn channels(&self) -> u16 {
        self.spec.channels.count() as u16
    }

    #[inline]
    fn sample_rate(&self) -> u32 {
        self.spec.rate
    }

    #[inline]
    fn total_duration(&self) -> Option<Duration> {
        Some(self.duration)
    }

    #[inline]
    fn elapsed(&mut self) -> Duration {
        self.elapsed
    }

    #[inline]
    fn fade_in_from_now(&mut self, duration: Duration) {
        let duration = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());

        self.fade_in_remaining_ns = duration as f32;
        self.fade_in_total_ns = duration as f32;
    }

    #[inline]
    fn fade_out_from_now(&mut self, duration: Duration) {
        let duration = duration.as_secs() * 1_000_000_000 + u64::from(duration.subsec_nanos());

        self.fade_out_remaining_ns = duration as f32;
        self.fade_out_total_ns = duration as f32;
    }

    #[inline]
    #[allow(clippy::cast_sign_loss, clippy::cast_possible_truncation)]
    fn seek(&mut self, time: Duration) -> Option<Duration> {
        // Suppresses noise at the moment of seek.
        self.fade_out_from_now(Duration::from_millis(10));
        self.is_seeking_soon = true;
        self.seek_to_time = time;
        Some(Duration::from_secs(0))
    }
}

impl Iterator for Symphonia {
    type Item = i16;

    #[inline]
    fn next(&mut self) -> Option<i16> {
        if self.current_frame_offset == self.buffer.len() {
            let mut decode_errors: usize = 0;
            let decoded = loop {
                match self.format.next_packet() {
                    Ok(packet) => match self.decoder.decode(&packet) {
                        Ok(decoded) => {
                            let ts = packet.ts();
                            if let Some(track) = self.format.default_track() {
                                if let Some(tb) = track.codec_params.time_base {
                                    let t = tb.calc_time(ts);
                                    self.elapsed = Duration::from_secs(t.seconds)
                                        + Duration::from_secs_f64(t.frac);
                                }
                            }
                            break decoded;
                        }
                        Err(e) => match e {
                            Error::DecodeError(_) => {
                                decode_errors += 1;
                                if decode_errors > MAX_DECODE_ERRORS {
                                    return None;
                                }
                            }
                            _ => return None,
                        },
                    },
                    Err(_) => return None,
                }
            };
            self.spec = *decoded.spec();
            self.buffer = Self::get_buffer(decoded, self.spec);
            self.current_frame_offset = 0;
        }

        let mut sample = self.buffer.samples()[self.current_frame_offset];
        self.current_frame_offset += 1;

        if self.fade_out_remaining_ns > 0.0 {
            let fade_out_factor =
                (self.fade_out_remaining_ns / self.fade_out_total_ns * 2.0 - 1.0).max(0.0);
            println!(
                "fade_out_remaining_ns: {:?}, fade_out_factor: {:?}",
                self.fade_out_remaining_ns, fade_out_factor
            );

            self.fade_out_remaining_ns -=
                1_000_000_000.0 / (self.sample_rate() as f32 * f32::from(self.channels()));

            sample = (sample as f32 * fade_out_factor) as i16;
        } else if self.is_seeking_soon {
            match self.format.seek(
                SeekMode::Coarse,
                // Symphonia seems to seek about 0.05s before the specified second (not earlier than 0.05s).
                // Then, at the moment of seek, the time 1 second earlier is displayed on the UI.
                // To solve this problem, seek to the time that adds 0.05s.
                SeekTo::Time {
                    time: Time::new(self.seek_to_time.as_secs(), 0.05),
                    track_id: None,
                },
            ) {
                Ok(seeked_to) => {
                    let base = TimeBase::new(1, self.sample_rate());
                    let time = base.calc_time(seeked_to.actual_ts);
                    let duration =
                        Duration::from_secs(time.seconds) + Duration::from_secs_f64(time.frac);
                    self.elapsed = duration;

                    // Suppresses noise at the moment of seek.
                    self.fade_in_from_now(Duration::from_millis(100));
                    // Some(duration)
                }
                Err(_) => {}
            }

            self.is_seeking_soon = false;
        }

        if self.fade_in_remaining_ns > 0.0 {
            let fade_in_factor =
                ((1.0 - self.fade_in_remaining_ns / self.fade_in_total_ns) * 2.0 - 1.0).max(0.0);
            println!(
                "fade_in_remaining_ns: {:?}, fade_in_factor: {:?}",
                self.fade_in_remaining_ns, fade_in_factor
            );

            self.fade_in_remaining_ns -=
                1_000_000_000.0 / (self.sample_rate() as f32 * f32::from(self.channels()));

            sample = (sample as f32 * fade_in_factor) as i16;
        }

        Some(sample)
    }
}

/// Error that can happen when creating a decoder.
#[derive(Debug, Clone)]
pub enum SymphoniaDecoderError {
    /// The format of the data has not been recognized.
    UnrecognizedFormat,

    /// An IO error occured while reading, writing, or seeking the stream.
    IoError(String),

    /// The stream contained malformed data and could not be decoded or demuxed.
    DecodeError(&'static str),

    /// A default or user-defined limit was reached while decoding or demuxing the stream. Limits
    /// are used to prevent denial-of-service attacks from malicious streams.
    LimitError(&'static str),

    /// The demuxer or decoder needs to be reset before continuing.
    ResetRequired,

    /// No streams were found by the decoder
    NoStreams,
}

impl fmt::Display for SymphoniaDecoderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::UnrecognizedFormat => "Unrecognized format",
            Self::IoError(msg) => &msg[..],
            Self::DecodeError(msg) | Self::LimitError(msg) => msg,
            Self::ResetRequired => "Reset required",
            Self::NoStreams => "No streams",
        };
        write!(f, "{}", text)
    }
}
impl std::error::Error for SymphoniaDecoderError {}
