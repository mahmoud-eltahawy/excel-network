use models::{Sheet, SheetShearchParams, Name};
use reqwest::StatusCode;

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

pub async fn search_for_5_sheets(
    app_state: &AppState,
    params: &SheetShearchParams,
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