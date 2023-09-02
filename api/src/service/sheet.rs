use crate::{column::save_cloumn_value, AppState};
use actix_web::{
    get, post, put,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};
use sqlx::{query, query_as};
use std::{collections::HashMap, error::Error};
use uuid::Uuid;

use models::{Column, Name, Row, SearchSheetParams, Sheet};

pub fn scope() -> Scope {
    web::scope("/sheet")
        .service(get_sheet_by_id)
        .service(search)
        .service(save)
        .service(update_name)
        .service(delete_sheet_rows)
        .service(add_rows_to_sheet)
        .service(get_number_of_sheet_rows_by_id)
        .service(get_custom_sheet_by_id)
}

#[post("/search")]
async fn search(state: Data<AppState>, params: web::Json<SearchSheetParams>) -> impl Responder {
    match search_by_params(&state, params.into_inner()).await {
        Ok(dep) => HttpResponse::Ok().json(dep),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[post("/")]
async fn save(state: Data<AppState>, sheet: web::Json<Sheet>) -> impl Responder {
    match save_sheet(&state, sheet.into_inner()).await {
        Ok(_) => HttpResponse::Ok().into(),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[put("/name")]
async fn update_name(state: Data<AppState>, name: web::Json<Name>) -> impl Responder {
    match update_sheet_name(&state, name.into_inner()).await {
        Ok(_) => HttpResponse::Ok().into(),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[get("/{id}")]
async fn get_sheet_by_id(state: Data<AppState>, id: web::Path<Uuid>) -> impl Responder {
    match fetch_sheet_by_id(&state, id.into_inner()).await {
        Ok(dep) => HttpResponse::Ok().json(dep),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[get("/{id}/{limit}")]
async fn get_custom_sheet_by_id(
    state: Data<AppState>,
    path: web::Path<(Uuid, i64)>,
) -> impl Responder {
    let (id, limit) = path.into_inner();
    match fetch_custom_sheet_by_id(&state, id, limit).await {
        Ok(dep) => HttpResponse::Ok().json(dep),
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[get("/{id}/{offset}/{limit}")]
async fn get_number_of_sheet_rows_by_id(
    state: Data<AppState>,
    path: web::Path<(Uuid, i64, i64)>,
) -> impl Responder {
    let (id, offset, limit) = path.into_inner();
    match fetch_rows_ids_by_sheet_id_in_limit(&state, &id, offset, limit).await {
        Ok(ids) => {
            let mut rows = Vec::new();
            for id in ids {
                if let Ok(columns) = fetch_columns_by_row_id(&state, &id).await {
                    rows.push(Row { id, columns })
                };
            }

            HttpResponse::Ok().json(rows)
        }
        Err(err) => HttpResponse::InternalServerError().json(err.to_string()),
    }
}

#[post("/rows")]
async fn add_rows_to_sheet(
    state: Data<AppState>,
    rows: web::Json<(Uuid, Vec<Row>)>,
) -> impl Responder {
    let (sheet_id, rows) = rows.into_inner();
    for row in rows {
        if let Err(err) = save_row(&state, &sheet_id, row).await {
            return HttpResponse::InternalServerError().json(err.to_string());
        }
    }
    HttpResponse::Ok().into()
}

#[post("/delete/rows")]
async fn delete_sheet_rows(
    state: Data<AppState>,
    ids: web::Json<(Uuid, Vec<Uuid>)>,
) -> impl Responder {
    let (sheet_id, rows_id) = ids.into_inner();
    for row_id in rows_id {
        if let Err(err) = delete_row_by_id(&state, &sheet_id, row_id).await {
            return HttpResponse::InternalServerError().json(err.to_string());
        }
    }
    HttpResponse::Ok().into()
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
        map.insert(
            record.header_name,
            Column {
                is_basic: true,
                value: serde_json::from_value(record.value)?,
            },
        );
    }
    Ok(map)
}

async fn fetch_rows_ids_by_sheet_id_in_limit(
    state: &AppState,
    sheet_id: &Uuid,
    offset: i64,
    limit: i64,
) -> Result<Vec<Uuid>, Box<dyn Error>> {
    let records = query!(
        r#"
        select id
        from rows WHERE sheet_id = $1 OFFSET $2 LIMIT $3"#,
        sheet_id,
        offset,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(records.into_iter().map(|x| x.id).collect())
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

pub async fn delete_row_by_id(
    state: &AppState,
    sheet_id: &Uuid,
    row_id: Uuid,
) -> Result<(), Box<dyn Error>> {
    query!(
        r#"
        DELETE FROM rows 
        WHERE sheet_id = $1 AND id = $2"#,
        sheet_id,
        row_id,
    )
    .execute(&state.db)
    .await?;
    Ok(())
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

async fn fetch_custom_sheet_by_id(
    state: &AppState,
    id: Uuid,
    limit: i64,
) -> Result<Sheet, Box<dyn Error>> {
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
    for id in fetch_rows_ids_by_sheet_id_in_limit(state, &id, 0, limit)
        .await?
        .into_iter()
    {
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
        if column.is_basic {
            save_cloumn_value(state, &Uuid::new_v4(), &id, header_name, column.value).await?;
        }
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

async fn update_sheet_name(state: &AppState, name: Name) -> Result<(), Box<dyn Error>> {
    let Name { id, the_name } = name;
    query!(
        r#"
        UPDATE sheets SET sheet_name = $2 WHERE id = $1;"#,
        id,
        the_name,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}
