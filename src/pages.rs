use actix_web::web;

mod auth;
mod attendance;

pub fn config(cfg: &mut web::ServiceConfig) {
    cfg
        .service(web::scope("/auth")
            .configure(auth::config))
        .service(web::scope("/attendance")
            .configure(attendance::config));
}
