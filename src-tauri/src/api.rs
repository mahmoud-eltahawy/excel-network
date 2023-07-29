use models::{Name, SearchSheetParams, Sheet, Row};
use reqwest::StatusCode;
use uuid::Uuid;

use crate::AppState;

pub async fn save_sheet(
    app_state: &AppState,
    sheet: &Sheet,
) -> Result<(), Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/"))
        .json(sheet)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        Err("failed".into())
    }
}

pub async fn update_sheet_name(
    app_state: &AppState,
    name :&Name
) -> Result<(), Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .put(format!("{origin}/sheet/name"))
        .json(name)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        Err("failed".into())
    }
}

pub async fn add_row_to_sheet(
    app_state: &AppState,
    sheet_id: &Uuid,
    row: &Row,
) -> Result<(), Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .post(format!("{origin}/sheet/{sheet_id}/row"))
        .json(row)
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        Err("failed".into())
    }
}

pub async fn delete_row_from_sheet(
    app_state: &AppState,
    sheet_id: &Uuid,
    row_id: &Uuid,
) -> Result<(), Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let res = reqwest::Client::new()
        .delete(format!("{origin}/sheet/{sheet_id}/{row_id}/row"))
        .send()
        .await?;

    if res.status() == StatusCode::OK {
        Ok(())
    } else {
        Err("failed".into())
    }
}

pub async fn search_for_5_sheets(
    app_state: &AppState,
    params: &SearchSheetParams,
) -> Result<Vec<Name>, Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let names = reqwest::Client::new()
        .post(format!("{origin}/sheet/search"))
        .json(params)
        .send()
        .await?
        .json::<Vec<Name>>()
        .await?;

    Ok(names)
}

pub async fn get_sheet_by_id(
    app_state: &AppState,
    id: &Uuid,
) -> Result<Sheet, Box<dyn std::error::Error>> {
    let origin = &app_state.origin;
    let sheet = reqwest::Client::new()
        .get(format!("{origin}/sheet/{}", id))
        .send()
        .await?
        .json::<Sheet>()
        .await?;

    Ok(sheet)
}
