// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod api;

use chrono::{Local, NaiveDate};
use dotenv::dotenv;
use models::{
    BackendColumnValue, BackendSheet, Column, ColumnId, ColumnValue, Config, ConfigValue,
    ImportConfig, Name, Row, RowIdentity, SearchSheetParams, Sheet, SheetConfig,
};
use std::{
    collections::HashMap,
    env,
    fs::{create_dir_all, rename, File},
    io::BufRead,
    path::Path,
    sync::Arc,
};
use uuid::Uuid;

use rust_xlsxwriter::{Color, Format, FormatBorder, Workbook};

use serde_json::{Deserializer, Value};

#[tauri::command]
fn sheets_types_names(sheets_types_names: tauri::State<'_, SheetsTypesNames>) -> Vec<Name> {
    sheets_types_names.0.clone()
}

#[tauri::command]
fn new_id() -> Uuid {
    Uuid::new_v4()
}

#[tauri::command]
fn sheet_type_name(
    sheets_types_names: tauri::State<'_, SheetsTypesNames>,
    id: Option<Uuid>,
) -> String {
    match id {
        Some(id) => sheets_types_names
            .0
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
fn sheet_primary_headers(
    sheet_import: tauri::State<'_, SheetImport>,
    name: Option<Arc<str>>,
) -> Vec<String> {
    match name {
        Some(name) => sheet_import
            .0
            .get(&name)
            .expect(&format!("expected name ({}) to exist", name))
            .primary
            .keys()
            .map(|x| x.clone())
            .collect::<Vec<_>>(),
        None => vec![],
    }
}

#[tauri::command]
fn sheet_headers(
    sheets_rows: tauri::State<'_, SheetsRows>,
    name: Option<Arc<str>>,
) -> Vec<ConfigValue> {
    match name {
        Some(name) => sheets_rows
            .0
            .get(&name)
            .expect(&format!("expected name ({}) to exist", name))
            .to_vec(),
        None => vec![],
    }
}

#[tauri::command]
async fn save_sheet(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    sheetname: String,
    typename: String,
    rows: Vec<Row>,
) -> Result<(), String> {
    if sheetname.is_empty() {
        return Err("اسم الشيت مطلوب".to_string());
    }
    let sheet = Sheet {
        id: sheetid,
        sheet_name: sheetname,
        type_name: typename,
        insert_date: Local::now().date_naive(),
        rows,
    };
    match api::save_sheet(&app_state, sheet).await {
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
) -> Result<(Sheet, i64), String> {
    match id {
        Some(id) => match api::get_custom_sheet_by_id(&app_state, &id, 10).await {
            Ok((sheet, len)) => Ok((sheet, len)),
            Err(err) => Err(err.to_string()),
        },
        None => Err("id is none".to_string()),
    }
}

#[tauri::command]
async fn get_sheet_rows(
    app_state: tauri::State<'_, AppState>,
    id: Option<Uuid>,
    offset: i64,
    limit: i64,
) -> Result<Vec<Row>, String> {
    match id {
        Some(id) => match api::get_sheet_rows_between(&app_state, &id, offset, limit).await {
            Ok(rows) => Ok(rows),
            Err(err) => Err(err.to_string()),
        },
        None => Err("id is none".to_string()),
    }
}

#[tauri::command]
async fn get_rows_ids(
    sheet_rows_ids: tauri::State<'_, SheetRowsIds>,
    name: Option<Arc<str>>,
) -> Result<RowIdentity, String> {
    let Some(name) = name else {
        return Err("id does not exist".to_string());
    };
    match sheet_rows_ids.0.get(&name) {
        Some(result) => Ok(result.clone()),
        None => Err("id does not exist".to_string()),
    }
}

#[tauri::command]
async fn get_priorities(
    priorities: tauri::State<'_, HashMap<Arc<str>, Arc<[Arc<str>]>>>,
    name: Option<Arc<str>>,
) -> Result<Arc<[Arc<str>]>, String> {
    let Some(name) = name else {
        return Ok(Arc::from([]));
    };
    match priorities.get(&name) {
        Some(list) => Ok(list.clone()),
        None => Err("priority does not exist".to_string()),
    }
}

#[tauri::command]
async fn update_sheet_name(
    app_state: tauri::State<'_, AppState>,
    name: Name,
) -> Result<(), String> {
    match api::update_sheet_name(&app_state, name).await {
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
    match api::add_rows_to_sheet(&app_state, sheetid, rows).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn delete_rows_from_sheet(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    rowsids: Vec<Uuid>,
) -> Result<(), String> {
    match api::delete_rows_from_sheet(&app_state, sheetid, rowsids).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn delete_columns(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    rows_headers: Vec<(Uuid, String)>,
) -> Result<(), String> {
    let columns_ids = rows_headers
        .into_iter()
        .map(|(row_id, header)| ColumnId {
            sheet_id: sheetid,
            row_id,
            header,
        })
        .collect::<Vec<_>>();
    match api::delete_columns(&app_state, columns_ids).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn save_columns(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    columnsidentifiers: Vec<(Uuid, String, ColumnValue)>,
) -> Result<(), String> {
    let columns_ids = columnsidentifiers
        .into_iter()
        .map(|(row_id, header, value)| {
            (
                ColumnId {
                    sheet_id: sheetid,
                    row_id,
                    header,
                },
                value,
            )
        })
        .collect::<Vec<_>>();
    match api::save_columns(&app_state, columns_ids).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn update_columns(
    app_state: tauri::State<'_, AppState>,
    sheetid: Uuid,
    columnsidentifiers: Vec<(Uuid, String, ColumnValue)>,
) -> Result<(), String> {
    let columns_ids = columnsidentifiers
        .into_iter()
        .map(|(row_id, header, value)| {
            (
                ColumnId {
                    sheet_id: sheetid,
                    row_id,
                    header,
                },
                value,
            )
        })
        .collect::<Vec<_>>();
    match api::update_columns(&app_state, columns_ids).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

#[tauri::command]
async fn export_sheet(headers: Arc<[Arc<str>]>, sheet: BackendSheet) -> Result<(), String> {
    match write_sheet(headers, sheet).await {
        Ok(_) => Ok(()),
        Err(err) => Err(err.to_string()),
    }
}

fn get_main_json_entry<'a>(json: &'a Value, entry: &Vec<String>) -> &'a Value {
    if entry.is_empty() {
        return json;
    }
    let mut json = json;
    for i in 0..entry.len() {
        json = json.get(entry[i].clone()).unwrap_or(&Value::Null);
    }
    json
}

fn column_from_value(value: &Value) -> Column {
    Column {
        is_basic: true,
        value: match value {
            Value::Number(_) => {
                ColumnValue::Float(serde_json::from_value(value.to_owned()).unwrap_or(0.0))
            }
            Value::String(v) => match v.parse::<f64>() {
                Ok(v) => ColumnValue::Float(v),
                Err(_) => match v
                    .get(..10)
                    .unwrap_or("unparsable string to date")
                    .parse::<NaiveDate>()
                {
                    Ok(v) => ColumnValue::Date(Some(v)),
                    Err(_) => ColumnValue::String(Some(v.to_owned())),
                },
            },
            _ => ColumnValue::String(Some("".to_string())),
        },
    }
}

#[tauri::command]
async fn import_sheet(
    sheet_import: tauri::State<'_, SheetImport>,
    sheettype: Arc<str>,
    sheetid: Uuid,
    filepath: String,
) -> Result<Vec<Row>, String> {
    let ImportConfig {
        main_entry,
        repeated_entry,
        unique,
        repeated,
        primary,
    } = match sheet_import.0.get(&sheettype) {
        Some(v) => v,
        None => return Ok(vec![]),
    };
    let Ok(file) = File::open(&filepath) else {
        return Ok(vec![]);
    };
    let reader = Deserializer::from_reader(file);
    let Some(Ok(main_json)) = reader.into_iter::<Value>().next() else {
        return Ok(vec![]);
    };
    let main_json = get_main_json_entry(&main_json, main_entry);
    let main_json = match main_json {
        Value::String(s) => serde_json::from_str(s).unwrap_or(Value::Null),
        _ => main_json.clone(),
    };
    let mut unique_columns = HashMap::new();
    for (header, entry) in unique.into_iter() {
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
        for (header, entry) in repeated.into_iter() {
            let value = get_main_json_entry(value, entry);
            let column = column_from_value(value);
            columns.insert(header.to_owned(), column);
        }
        result.push(Row {
            id: Uuid::new_v4(),
            columns,
        });
    }

    let mut primary_row = HashMap::new();
    for (header, entry) in primary.into_iter() {
        let value = get_main_json_entry(&main_json, entry);
        let column = column_from_value(value);
        primary_row.insert(header.to_owned(), column);
    }
    result.push(Row {
        id: sheetid,
        columns: primary_row,
    });

    let old_path = Path::new(&filepath);
    let download_dir = dirs::home_dir().unwrap_or_default().join("Downloads");
    let new_path = download_dir
        .join(WORKDIR)
        .join(&sheettype.to_string())
        .join("الملفات المستوردة");

    if !create_dir_all(new_path.clone()).is_ok() {
        println!("Directory already exists");
    }

    let new_path = new_path.join(old_path.file_name().unwrap_or_default());

    if download_dir == old_path.parent().unwrap_or(old_path) {
        if !rename(old_path, new_path).is_ok() {
            println!("failed to move file");
        };
    };

    Ok(result)
}

struct SheetsTypesNames(Vec<Name>);
struct SheetsRows(HashMap<Arc<str>, Vec<ConfigValue>>);
struct SheetImport(HashMap<Arc<str>, ImportConfig>);
struct SheetRowsIds(HashMap<Arc<str>, RowIdentity>);

use std::io::{BufReader, Cursor};

fn main() {
    dotenv().ok();

    let file_path = "config";
    let file = File::open(file_path).expect("config file does not exist");
    let mut reader = BufReader::new(file);

    let buf = reader.fill_buf().unwrap();

    let v: ciborium::Value = ciborium::de::from_reader(Cursor::new(buf)).unwrap();
    let config: Config = v.deserialized().unwrap();

    let Config { priorities, sheets } = config;
    let sheets_types_names_vec = sheets
        .iter()
        .map(|x| Name {
            id: Uuid::new_v4(),
            the_name: x.sheet_type_name.to_string(),
        })
        .collect::<Vec<_>>();
    let mut sheet_map = HashMap::new();
    let mut sheet_import = HashMap::new();
    let mut sheet_rows_ids = HashMap::new();
    for SheetConfig {
        sheet_type_name,
        row,
        importing,
        row_identity,
    } in sheets.into_iter()
    {
        sheet_map.insert(sheet_type_name.clone(), row);
        sheet_import.insert(sheet_type_name.clone(), importing);
        sheet_rows_ids.insert(sheet_type_name, row_identity);
    }

    tauri::Builder::default()
        .manage(AppState::default())
        .manage(SheetsTypesNames(sheets_types_names_vec))
        .manage(SheetsRows(sheet_map))
        .manage(priorities)
        .manage(SheetImport(sheet_import))
        .manage(SheetRowsIds(sheet_rows_ids))
        .invoke_handler(tauri::generate_handler![
            sheets_types_names,
            sheet_primary_headers,
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
            import_sheet,
            get_priorities,
            get_rows_ids,
            delete_columns,
            save_columns,
            update_columns,
            get_sheet_rows,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

pub struct AppState {
    pub origin: String,
}

impl Default for AppState {
    fn default() -> Self {
        let host = env::var("ERA_HOST").expect("invalid host key");
        let port = env::var("ERA_PORT").expect("invalid port key");

        AppState {
            origin: format!("http://{host}:{port}"),
        }
    }
}

pub async fn write_sheet(
    headers: Arc<[Arc<str>]>,
    sheet: BackendSheet,
) -> Result<(), Box<dyn std::error::Error>> {
    let BackendSheet {
        id,
        sheet_name,
        type_name,
        insert_date,
        rows,
    } = sheet;
    let mut workbook = Workbook::new();

    let primary_row = rows
        .iter()
        .filter(|x| x.id == id)
        .collect::<Vec<_>>()
        .first()
        .map(|x| x.columns.clone())
        .unwrap_or_default();

    let second_row_index = primary_row.len() + 1;

    let worksheet = workbook.add_worksheet();

    for (row, (header, column)) in primary_row.into_iter().enumerate() {
        let row = row as u32;
        worksheet.set_row_height(row, 35)?;

        worksheet.set_row_format(
            row,
            &Format::new()
                .set_background_color(Color::Cyan)
                .set_font_size(17)
                .set_reading_direction(2)
                .set_bold()
                .set_border(FormatBorder::Thin),
        )?;

        match column.value {
            BackendColumnValue::String(v) => {
                worksheet.write_string(row, 1, header.to_string())?;
                worksheet.write_string(row, 3, v.map(|x| x.to_string()).unwrap_or_default())?;
            }
            BackendColumnValue::Float(v) => {
                worksheet.write_string(row, 1, header.to_string())?;
                worksheet.write_number(row, 3, v)?;
            }
            BackendColumnValue::Date(v) => {
                let v = v.map(|x| x.to_string()).unwrap_or_default();
                worksheet.write_string(row, 1, header.to_string())?;
                worksheet.write_string(row, 3, v)?;
            }
        }
    }

    for (col, header) in headers.iter().enumerate() {
        let col = col as u16;
        worksheet.write_string(second_row_index as u32, col, header.to_string())?;
    }

    for (row, columns) in rows.into_iter().map(|x| x.columns).enumerate() {
        let row = row + second_row_index + 1;
        for (col, header) in headers.iter().enumerate() {
            let (row, col) = (row as u32, col as u16);
            worksheet.set_row_height(row, 30)?;
            match &columns.get(header) {
                Some(column) => match &column.value {
                    BackendColumnValue::Date(Some(date)) => {
                        let string = date.to_string();
                        worksheet.write_string(row, col, string)?;
                    }
                    BackendColumnValue::String(Some(string)) => {
                        worksheet.write_string(row, col, string.to_string())?;
                    }
                    BackendColumnValue::Float(number) => {
                        worksheet.write_number(row, col, *number)?;
                    }
                    _ => (),
                },
                None => (),
            }
        }
    }

    worksheet.set_row_height(second_row_index as u32, 45)?;
    worksheet.set_row_format(
        second_row_index as u32,
        &Format::new()
            .set_background_color(Color::Orange)
            .set_font_size(14)
            .set_reading_direction(2)
            .set_bold()
            .set_border(FormatBorder::DashDotDot),
    )?;
    worksheet.autofit();

    worksheet.set_right_to_left(true);

    worksheet.set_name(&type_name.to_string())?;

    let file_path = dirs::home_dir()
        .unwrap_or_default()
        .join("Downloads")
        .join(WORKDIR)
        .join(type_name.to_string())
        .join("الشيتات المصدرة");

    if !create_dir_all(file_path.clone()).is_ok() {
        println!("Directory already exists");
    }

    let file_name = format!(
        "_{}_{}_--_{}_{}_--_{}_{}.xlsx",
        "شيت",
        type_name,
        "باسم",
        sheet_name,
        "بتاريخ",
        insert_date.to_string(),
    );

    let path_name = file_path.join(file_name);

    workbook.save(&path_name)?;

    Ok(())
}

static WORKDIR: &str = "excel_network";
