#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use rodio::{source::Source, Decoder, OutputStream};
use std::fs::File;
use std::io::BufReader;
use tauri::Manager;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let id = app.listen_global("front-to-back", |event| {
                // Get a output stream handle to the default physical sound device
                let (_stream, stream_handle) = OutputStream::try_default().unwrap();
                // Load a sound from a file, using a path relative to Cargo.toml
                let file = BufReader::new(File::open(event.payload().unwrap()).unwrap());
                // Decode that sound file into a source
                let source = Decoder::new(file).unwrap();
                // Play the sound directly on the device
                stream_handle.play_raw(source.convert_samples());

                // The sound plays in a separate audio thread,
                // so we need to keep the main thread alive while it's playing.
                std::thread::sleep(std::time::Duration::from_secs(30));
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
