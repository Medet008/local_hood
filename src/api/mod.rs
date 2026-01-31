pub mod addresses;
pub mod admin;
pub mod announcements;
pub mod apartments;
pub mod auth;
pub mod chat;
pub mod cities;
pub mod communal;
pub mod complexes;
pub mod maintenance;
pub mod marketplace;
pub mod notifications;
pub mod osi;
pub mod security;
pub mod users;
pub mod voting;

use crate::middleware::AppState;
use axum::Router;

pub fn routes() -> Router<AppState> {
    Router::new()
        .nest("/auth", auth::routes())
        .nest("/users", users::routes())
        .nest("/cities", cities::routes())
        .nest("/addresses", addresses::routes())
        .nest("/complexes", complexes::routes())
        .nest("/apartments", apartments::routes())
        .nest("/osi", osi::routes())
        .nest("/security", security::routes())
        .nest("/announcements", announcements::routes())
        .nest("/marketplace", marketplace::routes())
        .nest("/votings", voting::routes())
        .nest("/communal", communal::routes())
        .nest("/notifications", notifications::routes())
        .nest("/chat", chat::routes())
        .nest("/maintenance", maintenance::routes())
        .nest("/admin", admin::routes())
}
