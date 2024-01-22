use std::io::Cursor;

use actix_web::web;
use serde::Deserialize;

pub mod column;
pub mod sheet;

fn extract<'a, T: Deserialize<'a>>(params: web::Bytes) -> Result<T, Box<dyn std::error::Error>> {
    let params = ciborium::de::from_reader::<ciborium::Value, _>(Cursor::new(params))
        .map(|body| body.deserialized::<T>())??;
    Ok(params)
}
