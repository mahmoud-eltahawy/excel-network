// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use dotenv::dotenv;
use uuid::Uuid;
use std::collections::HashMap;
use std::env;

use std::fs::File;

#[tauri::command]
fn sheets_types_names(
    app_state: tauri::State<'_, AppState>,
) -> Vec<Name> {
    app_state.sheets_types_names.clone()
}


#[tauri::command]
fn new_id() -> Uuid {
    Uuid::new_v4()
}

#[tauri::command]
fn sheet_type_name(
    app_state: tauri::State<'_, AppState>,
    id : Option<Uuid>,
) -> String {
    match id {
	Some(id) => app_state
	    .sheets_types_names
	    .clone()
	    .into_iter()
	    .filter(|x| x.id == id)
	    .collect::<Vec<_>>()
	    .first()
	    .expect("expected type name to exist")
	    .the_name
	    .clone(),
	None => String::from(""),
    }
    
}

#[tauri::command]
fn sheet_headers(
    app_state: tauri::State<'_, AppState>,
    name : Option<String>,
) -> Vec<ConfigValue> {
    match name {
	Some(name) => app_state.
	    sheet_map
	    .get(&name)
	    .expect(&format!("expected name ({}) to exist",name))
	    .to_vec(),
	None => vec![]
    }
    
}

fn main() {
    dotenv().ok();
    tauri::Builder::default()
	.manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
	    sheets_types_names,
	    sheet_headers,
	    sheet_type_name,
	    new_id
	])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub struct AppState {
    pub origin: String,
    pub sheets_types_names : Vec<Name>,
    pub sheet_map : HashMap<String,Vec<ConfigValue>>,
}

use models::{SheetConfig, Config, ConfigValue, Name};

impl Default for AppState {
    fn default() -> Self {
        let host = env::var("ERA_HOST").expect("invalid host key");
        let port = env::var("ERA_PORT").expect("invalid port key");

	let file_path = "config.json";
	let mut file = File::open(file_path).expect("config file does not exist");

	let config : Config = serde_json::from_reader(&mut file).unwrap();
	let Config { sheets } = config;
	let sheets_types_names = sheets
	    .iter()
	    .map(|x| Name {
		id: Uuid::new_v4(),
		the_name: x.sheet_type_name.clone()
	    })
	    .collect::<Vec<_>>();
	let mut sheet_map = HashMap::new();
	for SheetConfig { sheet_type_name, row } in sheets.into_iter() {
	    sheet_map.insert(sheet_type_name, row);
	}

        AppState {
            origin: format!("http://{host}:{port}"),
	    sheets_types_names,
	    sheet_map,
        }
    }
}
