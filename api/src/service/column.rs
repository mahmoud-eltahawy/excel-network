use actix_web::{post, put, web, HttpResponse, Responder, Scope};
use sqlx::query;

use std::error::Error;
use uuid::Uuid;

use std::io::Cursor;

use std::sync::Arc;

use crate::AppState;

use models::{ColumnId, ColumnValue};

pub fn scope() -> Scope {
    web::scope("/columns")
        .service(delete_columns)
        .service(update_columns)
        .service(save_columns)
}

#[post("/delete")]
async fn delete_columns(state: web::Data<AppState>, ids: web::Bytes) -> impl Responder {
    let ids = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(ids))
        .map(|body| body.deserialized::<Vec<ColumnId<Arc<str>>>>());

    let ids = match ids {
        Ok(Ok(ids)) => ids,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    for id in ids {
        if let Err(err) = delete_column_by_column_id(&state, id).await {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes());
        }
    }

    HttpResponse::Ok().into()
}

#[put("/")]
async fn update_columns(state: web::Data<AppState>, ids_and_values: web::Bytes) -> impl Responder {
    let ids_and_values =
        ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(ids_and_values))
            .map(|body| body.deserialized::<Vec<(ColumnId<Arc<str>>, ColumnValue<Arc<str>>)>>());

    let ids_and_values = match ids_and_values {
        Ok(Ok(ids_and_values)) => ids_and_values,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    for ids_and_value in ids_and_values {
        let (ids, value) = ids_and_value;
        if let Err(err) = update_column_by_column_id(&state, ids, value).await {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes());
        }
    }
    HttpResponse::Ok().into()
}

#[post("/")]
async fn save_columns(state: web::Data<AppState>, ids_and_values: web::Bytes) -> impl Responder {
    let ids_and_values =
        ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(ids_and_values))
            .map(|body| body.deserialized::<Vec<(ColumnId<Arc<str>>, ColumnValue<Arc<str>>)>>());

    let ids_and_values = match ids_and_values {
        Ok(Ok(ids_and_values)) => ids_and_values,
        Ok(Err(err)) => {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes())
        }
        Err(err) => return HttpResponse::InternalServerError().body(err.to_string().into_bytes()),
    };

    for ids_and_value in ids_and_values {
        let (
            ColumnId {
                sheet_id: _,
                row_id,
                header,
            },
            value,
        ) = ids_and_value;
        if let Err(err) = save_cloumn_value(&state, &row_id, header, value).await {
            return HttpResponse::InternalServerError().body(err.to_string().into_bytes());
        }
    }
    HttpResponse::Ok().into()
}

pub async fn delete_column_by_column_id(
    state: &AppState,
    ids: ColumnId<Arc<str>>,
) -> Result<(), Box<dyn Error>> {
    let ColumnId {
        sheet_id,
        row_id,
        header,
    } = ids;
    query!(
        r#"
        DELETE FROM columns 
            WHERE header_name = $1 AND row_id = (
                SELECT id FROM rows 
                    WHERE id = $2 AND sheet_id = $3
            )
        "#,
        header.to_string(),
        row_id,
        sheet_id,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

pub async fn update_column_by_column_id(
    state: &AppState,
    ids: ColumnId<Arc<str>>,
    value: ColumnValue<Arc<str>>,
) -> Result<(), Box<dyn Error>> {
    let ColumnId {
        sheet_id,
        row_id,
        header,
    } = ids;
    let value = serde_json::json!(value);
    query!(
        r#"
        UPDATE columns 
            set value = $1
            WHERE header_name = $2 AND row_id = (
                SELECT id FROM rows 
                    WHERE id = $3 AND sheet_id = $4
            )
        "#,
        value,
        header.to_string(),
        row_id,
        sheet_id,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}

pub async fn save_cloumn_value(
    state: &AppState,
    row_id: &Uuid,
    header_name: Arc<str>,
    value: ColumnValue<Arc<str>>,
) -> Result<(), Box<dyn Error>> {
    let column_id = Uuid::new_v4();
    let value = serde_json::json!(value);
    query!(
        r#"
	INSERT INTO columns(id,row_id,header_name,value)
	VALUES($1,$2,$3,$4)"#,
        column_id,
        row_id,
        header_name.to_string(),
        value,
    )
    .execute(&state.db)
    .await?;
    Ok(())
}
