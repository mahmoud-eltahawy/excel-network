mod config;
mod service;

use service::*;

use config::{get_config_postgres_url, get_configs_server, set_debug_configs};
use dotenv::dotenv;

use actix_web::{middleware::Logger, web::Data, App, HttpServer};

use sqlx::{postgres::PgPoolOptions, Pool, Postgres};

pub struct AppState {
    pub db: Pool<Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    set_debug_configs();

    let db_pool = connect_db_pool().await;

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(AppState {
                db: db_pool.clone(),
            }))
            .wrap(Logger::default())
            .service(sheet::scope())
    })
    .bind(get_configs_server())?
    .run()
    .await?;
    Ok(())
}

async fn connect_db_pool() -> Pool<Postgres> {
    let p = PgPoolOptions::new()
        .max_connections(10)
        .connect(&get_config_postgres_url())
        .await
        .expect("failed to connect db");

    sqlx::migrate!("db/migrations")
        .run(&p)
        .await
        .expect("migration failed");

    p
}
