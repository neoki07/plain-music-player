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
use std::sync::Mutex;
use tauri::State;
use anyhow::Result;

struct PlayerState(Mutex<Player>);

#[tauri::command]
fn play(path: &str, player: State<PlayerState>) -> Result<(), String> {
    println!("path: {:?}", path);
    player.0.lock().unwrap().play(Path::new(path));
    Ok(())
}

#[tauri::command]
fn get_progress(player: State<PlayerState>) -> (f64, i64, i64) {
    player.0.lock().unwrap().get_progress().unwrap()
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            play, get_progress
        ])
        .manage(PlayerState(Mutex::new(Player::new())))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
