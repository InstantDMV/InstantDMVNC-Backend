mod cache;
mod handlers;
mod models;
mod routes;
mod scraping;

use actix_web::{App, HttpServer};
use dotenv::dotenv;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();
    dotenv().expect("Failed to load .env file");

    HttpServer::new(|| App::new().configure(routes::init))
        .bind(("0.0.0.0", 80))?
        .run()
        .await
}
