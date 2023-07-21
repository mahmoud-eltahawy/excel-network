// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;
use std::collections::HashMap;
use std::env;

use std::fs::File;

#[tauri::command]
fn sheets_names(
    app_state: tauri::State<'_, AppState>,
) -> Vec<String> {
    app_state.sheets_names.clone()
}

#[tauri::command]
fn sheet_map(
    app_state: tauri::State<'_, AppState>,
) -> HashMap<String,Vec<ConfigValue>> {
    app_state.sheet_map.clone()
}

fn main() {
    dotenv().ok();
    tauri::Builder::default()
	.manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
	    sheets_names,
	    sheet_map,
	])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub struct AppState {
    pub origin: String,
    pub sheets_names : Vec<String>,
    pub sheet_map : HashMap<String,Vec<ConfigValue>>,
}

use models::{SheetConfig, Config, ConfigValue};

impl Default for AppState {
    fn default() -> Self {
        let host = env::var("ERA_HOST").expect("invalid host key");
        let port = env::var("ERA_PORT").expect("invalid port key");

	let file_path = "config.json";
	let mut file = File::open(file_path).expect("config file does not exist");

	let config : Config = serde_json::from_reader(&mut file).unwrap();
	let Config { sheets } = config;
	let sheets_names = sheets
	    .iter()
	    .map(|x| x.sheet_type_name.clone())
	    .collect::<Vec<_>>();
	let mut sheet_map = HashMap::new();
	for SheetConfig { sheet_type_name, row } in sheets.into_iter() {
	    sheet_map.insert(sheet_type_name, row);
	}

        AppState {
            origin: format!("http://{host}:{port}"),
	    sheets_names,
	    sheet_map,
        }
    }
}
