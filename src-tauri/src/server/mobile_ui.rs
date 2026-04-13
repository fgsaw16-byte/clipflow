use actix_web::{get, web, HttpResponse, Responder};
use tauri::{AppHandle, Emitter};
use crate::state::AppState;

const MOBILE_HTML: &str = include_str!("mobile.html");

#[get("/")]
pub async fn web_home(app: web::Data<AppHandle>, state: web::Data<AppState>) -> impl Responder {
    let _ = app.emit("mobile-connected", "connected");
    if let Ok(mut last) = state.last_content.lock() { *last = String::new(); }
    HttpResponse::Ok().content_type("text/html; charset=utf-8").body(MOBILE_HTML)
}
