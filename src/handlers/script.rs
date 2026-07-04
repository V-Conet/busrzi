use axum::{extract::State, http::HeaderMap, response::IntoResponse};

use crate::handlers::{AppState, CACHE_CONTROL_JS};

pub async fn get_script(State(state): State<AppState>) -> impl IntoResponse {
    let mut headers = HeaderMap::new();
    headers.insert(
        axum::http::header::CONTENT_TYPE,
        "application/javascript; charset=utf-8".parse().unwrap(),
    );
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        CACHE_CONTROL_JS.parse().unwrap(),
    );

    (headers, state.script_content)
}
