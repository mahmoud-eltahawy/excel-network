use models::{ColumnId, ColumnValue, Name, Row, SearchSheetParams, Sheet, ToSerial};
use reqwest::StatusCode;
use uuid::Uuid;

use std::io::Cursor;

use std::sync::Arc;

use crate::AppState;

pub async fn save_sheet(
    app_state: &AppState,
    sheet: Sheet<Uuid, Arc<str>>,
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
    name: Name<Uuid>,
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
    args: Vec<(ColumnId<Uuid, Arc<str>>, ColumnValue<Arc<str>>)>,
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
    args: Vec<(ColumnId<Uuid, Arc<str>>, ColumnValue<Arc<str>>)>,
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
    ids: Vec<ColumnId<Uuid, Arc<str>>>,
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
    rows: Vec<Row<Uuid, Arc<str>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let mut buffer = vec![];
    let rows = rows
        .into_iter()
        .map(|col| col.to_serial())
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&rows, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/{sheet_id}/rows"))
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
        .map(|col| col.to_serial())
        .collect::<Vec<_>>();
    ciborium::ser::into_writer(&rows, Cursor::new(&mut buffer))?;

    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/delete/{sheet_id}/rows"))
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
) -> Result<Vec<Name<Uuid>>, Box<dyn std::error::Error>> {
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
            .map(|body| body.deserialized::<Vec<Name<Uuid>>>().unwrap_or_default())
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
) -> Result<(Sheet<Uuid, Arc<str>>, i64), Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .get(format!("{origin}/sheet/{id}"))
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body))
            .map(|body| body.deserialized::<(Sheet<Uuid, Arc<str>>, i64)>());

        match body {
            Ok(Ok(v)) => Ok(v),
            Ok(Err(err)) => Err(err.to_string().into()),
            Err(err) => Err(err.to_string().into()),
        }
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}

pub async fn get_sheet_rows_between(
    app_state: &AppState,
    id: &Uuid,
    offset: i64,
    limit: i64,
) -> Result<Vec<Row<Uuid, Arc<str>>>, Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .get(format!("{origin}/sheet/{id}/{offset}/{limit}"))
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        let body = res.bytes().await.unwrap_or_default();
        let body = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(body))
            .map(|body| body.deserialized::<Vec<Row<Uuid, Arc<str>>>>());

        match body {
            Ok(Ok(rows)) => Ok(rows),
            Ok(Err(err)) => Err(err.to_string().into()),
            Err(err) => Err(err.to_string().into()),
        }
    } else {
        let body = res.bytes().await.unwrap_or_default();
        let body = String::from_utf8(body.to_vec()).unwrap_or_default();
        Err(body.into())
    }
}
