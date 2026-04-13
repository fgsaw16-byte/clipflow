pub mod handlers;
pub mod mobile_ui;

pub(crate) use handlers::broadcast_event;

pub fn configure_server(cfg: &mut actix_web::web::ServiceConfig) {
    cfg.service(mobile_ui::web_home)
       .service(handlers::sse_events)
       .service(handlers::receive_data)
       .service(handlers::receive_image)
       .service(handlers::receive_file);
}
