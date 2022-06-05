#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod player;

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::thread::sleep;
use std::time::Duration;
use tauri::Manager;
use player::Player;

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let id = app.listen_global("front-to-back", |event| {
                let mut player = Player::new();
                println!("event.payload: {:?}", event.payload().unwrap());
                player.play(Path::new(event.payload().unwrap()));

                let mut seconds = 0;

                for i in 0..10 {
                    println!("{} elapsed: {:?}", i, player.get_progress().unwrap());
                    sleep(Duration::new(0, 123456789));
                }

                let _ = player.seek_to(Duration::new(50, 0));
                println!("\n=============== seek ===============\n");

                for i in 0..10 {
                    println!("{} elapsed: {:?}", i, player.get_progress().unwrap());
                    sleep(Duration::new(0, 123456789));
                }

                let _ = player.seek_to(Duration::new(10, 0));
                println!("\n=============== seek ===============\n");

                for i in 0..10 {
                    println!("{} elapsed: {:?}", i, player.get_progress().unwrap());
                    sleep(Duration::new(0, 123456789));
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
