mod config;
mod service;

use service::*;

use config::{get_config_postgres_url, get_configs_server, set_debug_configs};
use dotenv::dotenv;

use actix_web::{middleware::Logger, web::Data, App, HttpServer};

use sqlx::{postgres::PgPoolOptions, query, Pool, Postgres};

pub struct AppState {
    pub db: Pool<Postgres>,
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    set_debug_configs();

    let db_pool = connect_db_pool().await;

    query!(
        r#"
          INSERT INTO rows(id,sheet_id) select s.id,s.id from sheets s 
          	where s.id not in (select r.id from "rows" r 
          		where r.id in (select id from sheets));"#,
    )
    .fetch_all(&db_pool)
    .await
    .expect("primary rows prepare failed");

    HttpServer::new(move || {
        App::new()
            .app_data(Data::new(AppState {
                db: db_pool.clone(),
            }))
            .wrap(Logger::default())
            .service(sheet::scope())
            .service(column::scope())
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
