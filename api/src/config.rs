use std::env;

pub fn set_debug_configs() {
    env::set_var("RUST_LOG", "debug");
    env::set_var("RUST_BACKTRACE", "1");
    env_logger::init();
}

pub fn get_configs_server() -> (String, u16) {
    let host = env::var("HOST").expect("invalid host key");
    let port: u16 = env::var("PORT")
        .expect("invalid port key")
        .parse()
        .expect("port must be an u16 number");
    (host, port)
}

pub fn get_config_postgres_url() -> String {
    env::var("DATABASE_URL").expect("unvalid db url key")
}
