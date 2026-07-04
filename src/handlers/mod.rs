pub mod log;
pub mod script;

use axum::http::{header, Method};
use crate::counter::CounterService;
use serde::Serialize;
use tower_http::cors::{Any, CorsLayer};

#[derive(Debug, Clone)]
pub struct AppState {
    pub counter: CounterService,
    pub script_content: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ApiResponse<T> {
    pub ok: bool,
    pub result: T,
    pub info: String,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn success(info: impl Into<String>, result: T) -> Self {
        Self {
            ok: true,
            result,
            info: info.into(),
        }
    }
}

pub fn cors_layer() -> CorsLayer {
    CorsLayer::new()
        .allow_origin(Any)
        .allow_methods([Method::GET, Method::HEAD, Method::POST, Method::OPTIONS])
        .allow_headers([header::CONTENT_TYPE])
        .max_age(std::time::Duration::from_secs(86400))
}

pub const CACHE_CONTROL_JS: &str = "public, max-age=86400, s-maxage=86400";
pub const CACHE_CONTROL_API: &str = "no-store, max-age=0";
