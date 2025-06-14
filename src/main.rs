use std::fs::OpenOptions;

use actix_web::{web, App, HttpServer};
use sea_orm::Database;
use tracing::Level;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{filter, fmt, layer::SubscriberExt, EnvFilter, Layer, Registry};

use crate::auth::Authority;

mod config;
mod consts;
mod utils;

mod entity;
mod auth;
mod pages;

#[actix_web::main]
async fn main() {
    let _ = dotenvy::dotenv();
    
    let log_file = OpenOptions::new()
        .append(true)
        .create(true)
        .open("trace.log")
        .unwrap();
    
    let subscriber = Registry::default()
        .with(
            fmt::layer()
                .with_ansi(true)
                .with_line_number(true)
                .with_filter(EnvFilter::from_default_env())
        )
        .with(
            fmt::layer()
                .with_ansi(false)
                .with_writer(log_file)
                .with_filter(filter::LevelFilter::from_level(Level::TRACE))
        );
    
    tracing::subscriber::set_global_default(subscriber).unwrap();
    
    let config::Config {
        host_address,
        database_opt,
        jwt_key
    } = config::load();
    
    let database = web::Data::new(Database::connect(database_opt).await.expect("Unable to connect to database"));
    let authority = web::Data::new(Authority::new(jwt_key.as_bytes()));

    let server = HttpServer::new(move || {
        App::new()
            .app_data(database.clone())
            .app_data(authority.clone())
            .wrap(TracingLogger::default())
            .configure(pages::config)
    });
    
    server
        .bind(host_address).unwrap()
        .run().await.unwrap();
}
