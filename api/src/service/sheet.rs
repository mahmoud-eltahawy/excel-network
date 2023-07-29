use crate::AppState;
use actix_web::{
    get, post,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};
use sqlx::{query, query_as};
use std::{collections::HashMap, error::Error};
use uuid::Uuid;

use models::{Column, Name, Row, SearchSheetParams, Sheet};

pub fn scope() -> Scope {
    web::scope("/sheet")
        .service(get_by_id)
        .service(search)
        .service(save)
}

pub async fn fetch_columns_by_row_id(
    state: &AppState,
    row_id: &Uuid,
) -> Result<HashMap<String, Column>, Box<dyn Error>> {
    let records = query!(
        r#"
        select header_name,value
        from columns WHERE row_id = $1"#,
        row_id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut map = HashMap::new();
    for record in records.into_iter() {
        map.insert(record.header_name, serde_json::from_value(record.value)?);
    }
    Ok(map)
}

pub async fn fetch_rows_ids_by_sheet_id(
    state: &AppState,
    sheet_id: &Uuid,
) -> Result<Vec<Uuid>, Box<dyn Error>> {
    let records = query!(
        r#"
        select id
        from rows WHERE sheet_id = $1"#,
        sheet_id,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(records.into_iter().map(|x| x.id).collect())
}

async fn fetch_sheet_by_id(state: &AppState, id: Uuid) -> Result<Sheet, Box<dyn Error>> {
    let record = query!(
        r#"
        select *
        from sheets WHERE id = $1"#,
        id
    )
    .fetch_one(&state.db)
    .await?;
    let id = record.id;
    let mut rows = Vec::new();
    for id in fetch_rows_ids_by_sheet_id(state, &id).await?.into_iter() {
        let columns = fetch_columns_by_row_id(state, &id).await?;
        rows.push(Row { id, columns });
    }
    Ok(Sheet {
        id,
        sheet_name: record.sheet_name,
        type_name: record.type_name,
        insert_date: record.insert_date,
        rows,
    })
}

#[get("/{id}")]
async fn get_by_id(state: Data<AppState>, id: web::Path<Uuid>) -> impl Responder {
    match fetch_sheet_by_id(&state, id.into_inner()).await {
        Ok(dep) => HttpResponse::Ok().json(dep),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

async fn search_by_params(
    state: &AppState,
    params: SearchSheetParams,
) -> Result<Vec<Name>, Box<dyn Error>> {
    let SearchSheetParams {
        offset,
        sheet_type_name,
        begin,
        end,
        sheet_name,
    } = params;
    let names = match (begin, end, sheet_name) {
        (None, None, None) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 OFFSET $2 LIMIT 5"#,
                sheet_type_name,
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (None, None, Some(sheet_name)) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND sheet_name LIKE $2
				OFFSET $3 LIMIT 5"#,
                sheet_type_name,
                format!("%{}%", sheet_name),
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (Some(begin), None, None) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND insert_date > $2
				OFFSET $3 LIMIT 5"#,
                sheet_type_name,
                begin,
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (Some(begin), None, Some(sheet_name)) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND
				(insert_date > $2 AND sheet_name LIKE $3)
				OFFSET $4 LIMIT 5"#,
                sheet_type_name,
                begin,
                format!("%{}%", sheet_name),
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (None, Some(end), None) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND insert_date < $2
				OFFSET $3 LIMIT 5"#,
                sheet_type_name,
                end,
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (None, Some(end), Some(sheet_name)) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND
				(insert_date < $2 AND sheet_name LIKE $3)
				OFFSET $4 LIMIT 5"#,
                sheet_type_name,
                end,
                format!("%{}%", sheet_name),
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (Some(begin), Some(end), None) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND insert_date BETWEEN $2 AND $3
				OFFSET $4 LIMIT 5"#,
                sheet_type_name,
                begin,
                end,
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
        (Some(begin), Some(end), Some(sheet_name)) => {
            query_as!(
                Name,
                r#"
				SELECT id,sheet_name as the_name
				FROM sheets WHERE type_name = $1 AND
				(insert_date BETWEEN $2 AND $3 AND sheet_name LIKE $4)
				OFFSET $5 LIMIT 5"#,
                sheet_type_name,
                begin,
                end,
                format!("%{}%", sheet_name),
                offset,
            )
            .fetch_all(&state.db)
            .await?
        }
    };
    Ok(names)
}

#[post("/search")]
async fn search(state: Data<AppState>, params: web::Json<SearchSheetParams>) -> impl Responder {
    match search_by_params(&state, params.into_inner()).await {
        Ok(dep) => HttpResponse::Ok().json(dep),
        Err(_) => HttpResponse::InternalServerError().into(),
    }
}

async fn save_cloumn(
    state: &AppState,
    row_id: &Uuid,
    header_name: String,
    column: Column,
) -> Result<(), Box<dyn Error>> {
    if !column.is_basic {
        return Ok(());
    }
    let id = Uuid::new_v4();
    let value = serde_json::json!(column);
    query!(
        r#"
	INSERT INTO columns(id,row_id,header_name,value)
	VALUES($1,$2,$3,$4)"#,
        id,
        row_id,
        header_name,
        value,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

async fn save_row(state: &AppState, sheet_id: &Uuid, row: Row) -> Result<(), Box<dyn Error>> {
    let Row { id, columns } = row;
    query!(
        r#"
	INSERT INTO rows(id,sheet_id)
	VALUES($1,$2)"#,
        id,
        sheet_id,
    )
    .execute(&state.db)
    .await?;
    for (header_name, column) in columns {
        save_cloumn(state, &id, header_name, column).await?;
    }
    Ok(())
}

async fn save_sheet(state: &AppState, sheet: Sheet) -> Result<(), Box<dyn Error>> {
    let Sheet {
        id,
        sheet_name,
        type_name,
        insert_date,
        rows,
    } = sheet;
    query!(
        r#"
	INSERT INTO sheets(id,sheet_name,type_name,insert_date)
	VALUES($1,$2,$3,$4)"#,
        id,
        sheet_name,
        type_name,
        insert_date,
    )
    .execute(&state.db)
    .await?;
    for row in rows {
        save_row(state, &id, row).await?;
    }
    Ok(())
}

#[post("/")]
async fn save(state: Data<AppState>, sheet: web::Json<Sheet>) -> impl Responder {
    match save_sheet(&state, sheet.into_inner()).await {
        Ok(_) => HttpResponse::Ok(),
        Err(_) => HttpResponse::InternalServerError(),
    }
}
