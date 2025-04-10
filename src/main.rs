mod cache;
mod handlers;
mod models;
mod routes;
mod scraping;

use actix_web::{App, HttpServer};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    tracing_subscriber::fmt::init();

    HttpServer::new(|| App::new().configure(routes::init))
        .bind(("0.0.0.0", 8080))?
        .run()
        .await
}
