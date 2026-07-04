use axum::{extract::State, Json};
use serde::Deserialize;
use tracing::{debug, warn};

use crate::{
    counter::{target_from_url, CounterData},
    error::AppError,
    handlers::{ApiResponse, AppState, CACHE_CONTROL_API},
};

#[derive(Debug, Deserialize)]
pub struct LogRequest {
    pub url: String,
    #[serde(default, alias = "isNewUv")]
    pub is_new_uv: bool,
}

pub async fn post_log(
    State(state): State<AppState>,
    Json(req): Json<LogRequest>,
) -> Result<(axum::http::HeaderMap, Json<ApiResponse<CounterData>>), AppError> {
    debug!(
        url = %req.url,
        is_new_uv = req.is_new_uv,
        "received log request"
    );

    let target = match target_from_url(&req.url) {
        Ok(t) => t,
        Err(e) => {
            warn!(url = %req.url, error = %e, "failed to parse request URL");
            return Err(e);
        }
    };

    let data = state.counter.record(&target, req.is_new_uv).await?;

    debug!(
        host = %target.host,
        path = %target.path,
        site_pv = data.site_pv,
        page_pv = data.page_pv,
        site_uv = data.site_uv,
        "counter updated"
    );

    let mut headers = axum::http::HeaderMap::new();
    headers.insert(
        axum::http::header::CACHE_CONTROL,
        CACHE_CONTROL_API.parse().unwrap(),
    );
    headers.insert(axum::http::header::PRAGMA, "no-cache".parse().unwrap());
    headers.insert(axum::http::header::EXPIRES, "0".parse().unwrap());

    Ok((headers, Json(ApiResponse::success("counters updated", data))))
}
