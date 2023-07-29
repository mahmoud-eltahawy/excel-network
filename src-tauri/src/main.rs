// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;

use chrono::Local;
use dotenv::dotenv;
use models::{ColumnValue, Config, ConfigValue, Name, Row, SearchSheetParams, Sheet, SheetConfig};
use std::collections::HashMap;
use std::env;
use uuid::Uuid;

use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};
use std::path::MAIN_SEPARATOR;

use std::fs::File;

#[tauri::command]
fn sheets_types_names(app_state: tauri::State<'_, AppState>) -> Vec<Name> {
    app_state.sheets_types_names.clone()
}

#[tauri::command]
fn new_id() -> Uuid {
    Uuid::new_v4()
}

#[tauri::command]
fn sheet_type_name(app_state: tauri::State<'_, AppState>, id: Option<Uuid>) -> String {
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
fn sheet_headers(app_state: tauri::State<'_, AppState>, name: Option<String>) -> Vec<ConfigValue> {
    match name {
        Some(name) => app_state
            .sheet_map
            .get(&name)
            .expect(&format!("expected name ({}) to exist", name))
            .to_vec(),
        None => vec![],
    }
}

#[tauri::command]
async fn save_sheet(
    app_state: tauri::State<'_, AppState>,
    sheetname: String,
    typename: String,
    rows: Vec<Row>,
) -> Result<(), String> {
    if sheetname.is_empty() {
        return Err("اسم الشيت مطلوب".to_string());
    }
    let sheet = Sheet {
        id: Uuid::new_v4(),
        sheet_name: sheetname,
        type_name: typename,
        insert_date: Local::now().date_naive(),
        rows,
    };
    match api::save_sheet(&app_state, &sheet).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn top_5_sheets(
    app_state: tauri::State<'_, AppState>,
    params: SearchSheetParams,
) -> Result<Vec<Name>, String> {
    match api::search_for_5_sheets(&app_state, &params).await {
        Ok(names) => Ok(names),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn get_sheet(
    app_state: tauri::State<'_, AppState>,
    id: Option<Uuid>,
) -> Result<Sheet, String> {
    match id {
        Some(id) => match api::get_sheet_by_id(&app_state, &id).await {
            Ok(sheet) => Ok(sheet),
            Err(err) => Err(err.to_string()),
        },
        None => Err("id is none".to_string()),
    }
}

#[tauri::command]
async fn update_sheet_name(
    app_state: tauri::State<'_, AppState>,
    name: Name,
) -> Result<(), String> {
    match api::update_sheet_name(&app_state, &name).await {
	Ok(_) => Ok(()),
	Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn add_rows_to_sheet(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    rows: Vec<Row>,
) -> Result<(), String> {
    let mut res = Ok(());
    for row in rows.into_iter(){
	if let Err(err) = api::add_row_to_sheet(&app_state, &sheetid,&row).await{
	    res = Err(err.to_string());
	    break;
	};
    }
    res
}

#[tauri::command]
async fn delete_rows_from_sheet(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    rowsids: Vec<Uuid>,
) -> Result<(), String> {
    let mut res = Ok(());
    for row_id in rowsids.into_iter(){
	if let Err(err) = api::delete_row_from_sheet(&app_state, &sheetid,&row_id).await{
	    res = Err(err.to_string());
	    break;
	};
    }
    res
}
#[tauri::command]
async fn export_sheet(headers: Vec<String>, sheet: Sheet) -> Result<(), String> {
    match write_sheet(headers, sheet).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
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
            new_id,
            save_sheet,
            top_5_sheets,
            get_sheet,
            export_sheet,
	    update_sheet_name,
	    add_rows_to_sheet,
	    delete_rows_from_sheet,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub struct AppState {
    pub origin: String,
    pub sheets_types_names: Vec<Name>,
    pub sheet_map: HashMap<String, Vec<ConfigValue>>,
}

impl Default for AppState {
    fn default() -> Self {
        let host = env::var("ERA_HOST").expect("invalid host key");
        let port = env::var("ERA_PORT").expect("invalid port key");

        let file_path = "config.json";
        let mut file = File::open(file_path).expect("config file does not exist");

        let config: Config = serde_json::from_reader(&mut file).unwrap();
        let Config { sheets } = config;
        let sheets_types_names = sheets
            .iter()
            .map(|x| Name {
                id: Uuid::new_v4(),
                the_name: x.sheet_type_name.clone(),
            })
            .collect::<Vec<_>>();
        let mut sheet_map = HashMap::new();
        for SheetConfig {
            sheet_type_name,
            row,
        } in sheets.into_iter()
        {
            sheet_map.insert(sheet_type_name, row);
        }

        AppState {
            origin: format!("http://{host}:{port}"),
            sheets_types_names,
            sheet_map,
        }
    }
}

pub async fn write_sheet(
    headers: Vec<String>,
    sheet: Sheet,
) -> Result<(), Box<dyn std::error::Error>> {
    let Sheet {
        id: _,
        sheet_name,
        type_name,
        insert_date,
        rows,
    } = sheet;
    let mut workbook = Workbook::new();

    let worksheet = workbook.add_worksheet();

    for (col, header) in headers.iter().enumerate() {
        let col = col as u16;
        worksheet.write_string(0, col, header)?;
    }

    for (row, columns) in rows.into_iter().map(|x| x.columns).enumerate() {
        for (col, header) in headers.iter().enumerate() {
            let (row, col) = (row as u32 + 1, col as u16);
            match &columns.get(header).unwrap().value {
                ColumnValue::Date(Some(date)) => {
                    let string = date.to_string();
                    worksheet.write_string(row, col, string)?
                }
                ColumnValue::String(Some(string)) => worksheet.write_string(row, col, string)?,
                ColumnValue::Float(number) => worksheet.write_number(row, col, *number)?,
                _ => worksheet.write_string(row, col, "فارغ")?,
            };
        }
    }

    worksheet.autofit();
    worksheet.set_row_height(0, 25)?;
    worksheet.set_row_format(
        0,
        &Format::new()
            .set_background_color(Color::Orange)
            .set_font_size(14)
            .set_reading_direction(2)
            .set_bold()
            .set_border(FormatBorder::DashDotDot),
    )?;

    worksheet.set_right_to_left(true);

    worksheet.set_name(&type_name)?;

    let file_path = format!(
        "{}{MAIN_SEPARATOR}Downloads{MAIN_SEPARATOR}",
        dirs::home_dir().unwrap_or_default().display()
    );

    let file_name = format!(
        "_{}_{}_--_{}_{}_--_{}_{}.xlsx",
        "شيت",
        type_name,
        "باسم",
        sheet_name,
        "بتاريخ",
        insert_date.to_string(),
    );
    let path_name = file_path + &file_name;
    workbook.save(&path_name)?;

    Ok(())
}
