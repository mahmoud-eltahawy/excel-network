// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;

use chrono::{Local, NaiveDate};
use dotenv::dotenv;
use models::{ColumnValue, Config, ConfigValue, Name, Row, SearchSheetParams, Sheet, SheetConfig, ImportConfig, Column};
use std::{env,fs::File,path::MAIN_SEPARATOR,collections::HashMap};
use uuid::Uuid;

use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};

use serde_json::{Value,Deserializer};

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

fn get_main_json_entry<'a>(
    json : &'a Value,
    entry : &Vec<String>,
) -> &'a Value{
    if entry.is_empty() {
	return json;
    }
    let mut json = json;
    for i in 0..entry.len() {
	json = json.get(entry[i].clone()).unwrap_or(&Value::Null);
    }
    json
}

fn column_from_value(value: &Value) -> Column{
    Column{
	is_basic : true,
	value : match value {
	    Value::Number(_) => ColumnValue::Float(serde_json::from_value(value.to_owned())
						.unwrap_or(0.0)),
	    Value::String(v) => {
		match v.parse::<f64>() {
		    Ok(v) => ColumnValue::Float(v),
		    Err(_) => match v[0..10].parse::<NaiveDate>() {
			Ok(v) => ColumnValue::Date(Some(v)),
			Err(_) => ColumnValue::String(Some(v.to_owned()))
		    }
		}
	    },
	    _ => ColumnValue::Float(0.0),
	}
    }
}

#[tauri::command]
async fn import_sheet(
    app_state: tauri::State<'_, AppState>,
    sheettype: String,
    name: String,
) -> Result<Vec<Row>, String> {
    let ImportConfig{
	main_entry,
	repeated_entry,
	unique,
	repeated,
    } = match app_state.sheet_import.get(&sheettype){
	Some(v) => v,
	None => return Ok(vec![]),
    };
    let full_path = app_state.import_path.clone() + &name;
    let Ok(file) = File::open(&full_path) else {
	return Ok(vec![]);
    };
    let reader = Deserializer::from_reader(file);
    let Some(Ok(main_json)) = reader.into_iter::<Value>().next() else {
	return Ok(vec![]);
    };
    let main_json = get_main_json_entry(&main_json, main_entry);
    let main_json = match main_json {
	Value::String(s) => {
	    serde_json::from_str(s).unwrap_or(Value::Null)
	},
	_ => main_json.clone(),
    };
    let mut unique_columns = HashMap::new();
    for (header,entry) in unique.into_iter() {
	let value = get_main_json_entry(&main_json, entry);
	let column = column_from_value(value);
	unique_columns.insert(header.to_owned(), column);
    }
    let repeated_json = get_main_json_entry(&main_json, repeated_entry);
    let Value::Array(list) = repeated_json else {
	return Ok(vec![]);
    };
    let mut result = Vec::new();
    for value in list.into_iter() {
	let mut columns = unique_columns.clone();
	for (header,entry) in repeated.into_iter() {
	    let value = get_main_json_entry(value, entry);
	    let column = column_from_value(value);
	    columns.insert(header.to_owned(), column);
	}
	result.push(Row{
	    id:Uuid::new_v4(),
	    columns
	});
    }
    Ok(result)
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
	    import_sheet
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub struct AppState {
    pub origin: String,
    pub import_path : String,
    pub sheets_types_names: Vec<Name>,
    pub sheet_map: HashMap<String, Vec<ConfigValue>>,
    pub sheet_import: HashMap<String, ImportConfig>,
}

impl Default for AppState {
    fn default() -> Self {
        let host = env::var("ERA_HOST").expect("invalid host key");
        let port = env::var("ERA_PORT").expect("invalid port key");

        let file_path = "config.json";
        let mut file = File::open(file_path).expect("config file does not exist");

        let config: Config = serde_json::from_reader(&mut file).unwrap();
        let Config { import_path,sheets } = config;
        let sheets_types_names = sheets
            .iter()
            .map(|x| Name {
                id: Uuid::new_v4(),
                the_name: x.sheet_type_name.clone(),
            })
            .collect::<Vec<_>>();
        let mut sheet_map = HashMap::new();
        let mut sheet_import = HashMap::new();
        for SheetConfig {
            sheet_type_name,
            row,
	    importing,
        } in sheets.into_iter()
        {
            sheet_map.insert(sheet_type_name.clone(), row);
            sheet_import.insert(sheet_type_name, importing);
        }

        AppState {
            origin: format!("http://{host}:{port}"),
            sheets_types_names,
            sheet_map,
	    import_path,
	    sheet_import,
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
        "شيت",type_name,
        "باسم",sheet_name,
        "بتاريخ",insert_date.to_string(),
    );
    let path_name = file_path + &file_name;
    workbook.save(&path_name)?;

    Ok(())
}
