use models::{ColumnId, ColumnValue, Name, NameSerial, Row, SearchSheetParams, Sheet, SheetSerial};
use reqwest::StatusCode;
use uuid::Uuid;

use std::io::Cursor;

use crate::AppState;

pub async fn save_sheet(
    app_state: &AppState,
    sheet: Sheet,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let sheet = sheet.to_serial();
    ciborium::ser::into_writer(&sheet, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body))
            .map(|body| body.deserialized::<String>().unwrap_or_default())
            .unwrap_or_default();
        Err(body.into())
    }
}

pub async fn update_sheet_name(
    app_state: &AppState,
    name: Name,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let name = name.to_serial();
    ciborium::ser::into_writer(&name, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .put(format!("{origin}/sheet/name"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body))
            .map(|body| body.deserialized::<String>().unwrap_or_default())
            .unwrap_or_default();
        Err(body.into())
    }
}

pub async fn update_columns(
    app_state: &AppState,
    args: Vec<(ColumnId, ColumnValue)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let args = args
        .into_iter()
        .map(|(col, val)| (col.to_serial(), val))
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&args, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .put(format!("{origin}/columns/"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn save_columns(
    app_state: &AppState,
    args: Vec<(ColumnId, ColumnValue)>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let args = args
        .into_iter()
        .map(|(col, val)| (col.to_serial(), val))
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&args, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/columns/"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn delete_columns(
    app_state: &AppState,
    ids: Vec<ColumnId>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let ids = ids
        .into_iter()
        .map(|col| col.to_serial())
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&ids, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/columns/delete"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn add_rows_to_sheet(
    app_state: &AppState,
    sheet_id: Uuid,
    rows: Vec<Row>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let rows = rows
        .into_iter()
        .map(|col| col.to_serial())
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&(sheet_id.to_string(), rows), Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/rows"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn delete_rows_from_sheet(
    app_state: &AppState,
    sheet_id: Uuid,
    rows: Vec<Uuid>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let rows = rows
        .into_iter()
        .map(|col| col.to_string())
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&(sheet_id.to_string(), rows), Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/delete/rows"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn search_for_5_sheets(
    app_state: &AppState,
    params: &SearchSheetParams,
) -> Result<Vec<Name>, Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    ciborium::ser::into_writer(&params, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/search"))
        .body(buffer)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body))
            .map(|body| {
                let body = body
                    .deserialized::<Vec<NameSerial>>()
                    .map(|names| {
                        names
                            .into_iter()
                            .flat_map(|name| name.to_origin())
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                body
            })
            .unwrap_or_default();
        Ok(body)
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn get_sheet_by_id(
    app_state: &AppState,
    id: &Uuid,
) -> Result<Sheet, Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .get(format!("{origin}/sheet/{}", id))
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body)).map(|body| {
            body.deserialized::<SheetSerial>()
                .unwrap_or_default()
                .to_origin()
        });

        match body {
            Ok(Ok(body)) => Ok(body),
            Ok(Err(err)) => Err(err.to_string().into()),
            Err(err) => Err(err.to_string().into()),
        }
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}
