pub mod health;
pub mod listen;
pub mod offices;

use actix_web::web;

pub fn init(cfg: &mut web::ServiceConfig) {
    cfg.service(web::scope("/health").configure(health::init))
        .service(web::scope("/offices").configure(offices::init))
        .service(web::scope("/listen").configure(listen::init));
}
