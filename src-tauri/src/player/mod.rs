#![cfg_attr(test, deny(missing_docs))]

mod conversions;
mod sink;
mod stream;

pub mod buffer;
pub mod decoder;
pub mod dynamic_mixer;
pub mod queue;
pub mod source;

pub use conversions::Sample;
pub use cpal::{
    self, traits::DeviceTrait, Device, Devices, DevicesError, InputDevices, OutputDevices,
    SupportedStreamConfig,
};
pub use decoder::Symphonia;
pub use sink::Sink;
pub use source::Source;
use std::fmt;
pub use stream::{OutputStream, OutputStreamHandle, PlayError, StreamError};

use std::fs::File;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

static VOLUME_STEP: u16 = 5;
static SEEK_STEP: f64 = 5.0;

pub struct Player {
    _stream: OutputStream,
    handle: OutputStreamHandle,
    sink: Sink,
    total_duration: Option<Duration>,
    is_stopped: bool,
    pub volume: u16,
    pub speed: f32,
    pub gapless: bool,
}

unsafe impl Send for Player {}

impl Player {
    pub fn new() -> Self {
        let (stream, handle) = OutputStream::try_default().unwrap();
        let gapless = true;
        let sink = Sink::try_new(&handle, gapless).unwrap();
        let volume = 5;
        sink.set_volume(f32::from(volume) / 100.0);
        let speed = 1.0;
        sink.set_speed(speed);

        Self {
            _stream: stream,
            handle,
            sink,
            total_duration: None,
            is_stopped: true,
            volume,
            speed,
            gapless,
        }
    }
    pub fn play(&mut self, path: &Path) {
        self.stop();
        if let Ok(file) = File::open(path) {
            if let Ok(decoder) = Symphonia::new(file, self.gapless) {
                self.total_duration = decoder.total_duration();
                self.sink.append(decoder);
                self.sink.set_speed(self.speed);
                self.is_stopped = false;
            }
        }
    }

    pub fn pause(&mut self) {
        self.sink.pause();
    }

    pub fn resume(&mut self) {
        self.sink.play();
    }

    pub fn is_paused(&self) -> bool {
        self.sink.is_paused()
    }

    pub fn stop(&mut self) {
        self.sink = Sink::try_new(&self.handle, self.gapless).unwrap();
        self.sink.set_volume(f32::from(self.volume) / 100.0);
        self.is_stopped = true;
    }
    pub fn elapsed(&self) -> Duration {
        self.sink.elapsed()
    }
    pub fn duration(&self) -> Option<f64> {
        self.total_duration
            .map(|duration| duration.as_secs_f64() - 0.29)
    }

    pub fn seek_fw(&mut self) {
        let new_pos = self.elapsed().as_secs_f64() + SEEK_STEP;
        if let Some(duration) = self.duration() {
            if new_pos < duration - SEEK_STEP {
                self.seek_to(Duration::from_secs_f64(new_pos));
            }
        }
    }
    pub fn seek_bw(&mut self) {
        let mut new_pos = self.elapsed().as_secs_f64() - SEEK_STEP;
        if new_pos < 0.0 {
            new_pos = 0.0;
        }

        self.seek_to(Duration::from_secs_f64(new_pos));
    }
    pub fn seek_to(&self, time: Duration) {
        self.sink.seek(time);
    }
    pub fn percentage(&self) -> f64 {
        self.duration().map_or(0.0, |duration| {
            let elapsed = self.elapsed();
            elapsed.as_secs_f64() / duration
        })
    }
    pub fn set_speed(&mut self, speed: f32) {
        self.speed = speed;
        self.sink.set_speed(speed);
    }

    pub fn get_progress(&mut self) -> Result<(f64, i64, i64), PlayerError> {
        if self.is_stopped {
            return Err(PlayerError::StoppedError);
        }

        let position = self.elapsed().as_secs() as i64;
        let duration = self.duration().unwrap() as i64;
        let mut percent = self.percentage() * 100.0;
        if percent > 100.0 {
            percent = 100.0;
        }
        Ok((percent, position, duration))
    }
}

#[derive(Debug, Serialize)]
pub enum PlayerError {
    StoppedError,
}
