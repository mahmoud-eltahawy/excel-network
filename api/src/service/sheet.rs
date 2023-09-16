use crate::{column::save_cloumn_value, AppState};
use actix_web::{
    get, post, put,
    web::{self, Data},
    HttpResponse, Responder, Scope,
};
use sqlx::{query, query_as};
use std::{collections::HashMap, error::Error};
use uuid::Uuid;

use std::sync::Arc;

use models::{
    Column, Name, NameSerial, Row, RowSerial, SearchSheetParams, Sheet, SheetSerial, ToOrigin,
    ToSerial,
};

use std::io::Cursor;

pub fn scope() -> Scope {
    web::scope("/sheet")
        .service(get_custom_sheet_by_id)
        .service(search)
        .service(save)
        .service(update_name)
        .service(delete_sheet_rows)
        .service(add_rows_to_sheet)
        .service(get_number_of_sheet_rows_by_id)
}

#[post("/search")]
async fn search(state: Data<AppState>, params: web::Bytes) -> impl Responder {
    let params = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(params))
        .map(|body| body.deserialized::<SearchSheetParams>());

    let params = match params {
        Ok(Ok(params)) => params,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    match search_by_params(&state, params).await {
        Ok(dep) => {
            let mut buf = vec![];
            let res = ciborium::ser::into_writer(
                &dep.into_iter()
                    .map(|name| name.to_serial())
                    .collect::<Vec<_>>(),
                Cursor::new(&mut buf),
            )
            .map(|_| HttpResponse::Ok().body(buf))
            .map_err(|err| HttpResponse::InternalServerError().body(err.to_string().into_bytes()));
            match res {
                Ok(fine) => fine,
                Err(err) => err,
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    }
}

#[post("/")]
async fn save(state: Data<AppState>, sheet: web::Bytes) -> impl Responder {
    let sheet = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(sheet)).map(|body| {
        body.deserialized::<SheetSerial<Arc<str>>>()
            .map(|sheet| sheet.to_origin())
    });

    let sheet = match sheet {
        Ok(Ok(Ok(sheet))) => sheet,
        Ok(Ok(Err(err))) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };
    match save_sheet(&state, sheet).await {
        Ok(_) => HttpResponse::Ok().into(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    }
}

#[put("/name")]
async fn update_name(state: Data<AppState>, name: web::Bytes) -> impl Responder {
    let name = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(name)).map(|body| {
        body.deserialized::<NameSerial>()
            .map(|name| name.to_origin())
    });

    let name = match name {
        Ok(Ok(Ok(name))) => name,
        Ok(Ok(Err(err))) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    match update_sheet_name(&state, name).await {
        Ok(_) => HttpResponse::Ok().into(),
        Err(err) => HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    }
}

#[get("/{id}/{limit}")]
async fn get_custom_sheet_by_id(
    state: Data<AppState>,
    path: web::Path<(Uuid, i64)>,
) -> impl Responder {
    let (id, limit) = path.into_inner();
    match (
        fetch_sheet_rows_length(&state, &id).await,
        fetch_custom_sheet_by_id(&state, id, limit).await,
    ) {
        (Ok(len), Ok(sheet)) => {
            let mut buf = vec![];
            let res = ciborium::ser::into_writer(&(sheet.to_serial(), len), Cursor::new(&mut buf))
                .map(|_| HttpResponse::Ok().body(buf))
                .map_err(|err| {
                    HttpResponse::InternalServerError().body(err.to_string().into_bytes())
                });
            match res {
                Ok(fine) => fine,
                Err(err) => err,
            }
        }
        (Err(err1), Err(err2)) => HttpResponse::InternalServerError()
            .body((err1.to_string() + " and " + &err2.to_string()).into_bytes()),
        (Err(err1), Ok(_)) => {
            HttpResponse::InternalServerError().body(err1.to_string().into_bytes())
        }
        (Ok(_), Err(err)) => HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
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

            let mut buf = vec![];
            let res = ciborium::ser::into_writer(
                &rows
                    .into_iter()
                    .map(|row| row.to_serial())
                    .collect::<Vec<_>>(),
                Cursor::new(&mut buf),
            )
            .map(|_| HttpResponse::Ok().body(buf))
            .map_err(|err| HttpResponse::InternalServerError().body(err.to_string().into_bytes()));
            match res {
                Ok(fine) => fine,
                Err(err) => err,
            }
        }
        Err(err) => HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    }
}

#[post("/{sheet_id}/rows")]
async fn add_rows_to_sheet(
    state: Data<AppState>,
    sheet_id: web::Path<Uuid>,
    rows: web::Bytes,
) -> impl Responder {
    let sheet_id = sheet_id.into_inner();

    let rows = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(rows)).map(|body| {
        body.deserialized::<Vec<RowSerial<Arc<str>>>>().map(|rows| {
            rows.into_iter()
                .flat_map(|x| x.to_origin())
                .collect::<Vec<_>>()
        })
    });

    let rows = match rows {
        Ok(Ok(rows)) => rows,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    for row in rows {
        if let Err(err) = save_row(&state, &sheet_id, row).await {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes());
        }
    }
    HttpResponse::Ok().into()
}

#[post("/delete/{sheet_id}/rows")]
async fn delete_sheet_rows(
    state: Data<AppState>,
    sheet_id: web::Path<Uuid>,
    rows: web::Bytes,
) -> impl Responder {
    let sheet_id = sheet_id.into_inner();
    let rows = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(rows)).map(|body| {
        body.deserialized::<Vec<Arc<str>>>().map(|rows| {
            rows.into_iter()
                .flat_map(|x| x.to_origin())
                .collect::<Vec<_>>()
        })
    });

    let rows = match rows {
        Ok(Ok(rows)) => rows,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    for row_id in rows {
        if let Err(err) = delete_row_by_id(&state, &sheet_id, row_id).await {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes());
        }
    }
    HttpResponse::Ok().into()
}

pub async fn fetch_columns_by_row_id(
    state: &AppState,
    row_id: &Uuid,
) -> Result<HashMap<Arc<str>, Column<Arc<str>>>, Box<dyn Error>> {
    let records = query!(
        r#"
        select header_name,value
        from columns WHERE row_id = $1"#,
        row_id,
    )
    .fetch_all(&state.db)
    .await?;
    let mut map = HashMap::<Arc<str>, Column<Arc<str>>>::new();
    for record in records.into_iter() {
        map.insert(
            Arc::from(record.header_name),
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
        from rows WHERE sheet_id = $1 ORDER BY insert_date OFFSET $2 LIMIT $3"#,
        sheet_id,
        offset,
        limit,
    )
    .fetch_all(&state.db)
    .await?;
    Ok(records.into_iter().map(|x| x.id).collect())
}

async fn fetch_sheet_rows_length(state: &AppState, sheet_id: &Uuid) -> Result<i64, Box<dyn Error>> {
    let records = query!(
        r#"
        select count(id) as len
        from rows WHERE sheet_id = $1"#,
        sheet_id,
    )
    .fetch_one(&state.db)
    .await?;
    Ok(records.len.unwrap_or_default())
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

async fn fetch_custom_sheet_by_id(
    state: &AppState,
    id: Uuid,
    limit: i64,
) -> Result<Sheet<Arc<str>>, Box<dyn Error>> {
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
        sheet_name: Arc::from(record.sheet_name),
        type_name: Arc::from(record.type_name),
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

async fn save_row(
    state: &AppState,
    sheet_id: &Uuid,
    row: Row<Arc<str>>,
) -> Result<(), Box<dyn Error>> {
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
            save_cloumn_value(state, &id, header_name, column.value).await?;
        }
    }
    Ok(())
}

async fn save_sheet(state: &AppState, sheet: Sheet<Arc<str>>) -> Result<(), Box<dyn Error>> {
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
        sheet_name.to_string(),
        type_name.to_string(),
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
