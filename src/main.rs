mod config;
mod counter;
mod error;
mod handlers;

use axum::{
    Router,
    routing::{get, post},
};
use tokio::net::TcpListener;
use tower_http::trace::TraceLayer;
use tracing::info;

use crate::{
    config::Config,
    counter::CounterService,
    handlers::{AppState, cors_layer},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let config = Config::from_env()?;

    init_tracing();

    info!(
        addr = %config.addr,
        redis_url = %config.redis_url,
        script_path = %config.script_path,
        ttl_days = config.ttl_days,
        "starting busrzi v{}",
        env!("CARGO_PKG_VERSION")
    );

    let redis_client = redis::Client::open(config.redis_url.as_str())?;
    let redis_manager = redis::aio::ConnectionManager::new(redis_client).await?;

    redis::cmd("PING")
        .query_async::<()>(&mut redis_manager.clone())
        .await?;
    info!("redis connected");

    let script_content = tokio::fs::read_to_string(&config.script_path).await?;

    let app_state = AppState {
        counter: CounterService::new(redis_manager, config.ttl_days),
        script_content,
    };

    let app = Router::new()
        .route("/api/counter", post(handlers::log::post_log))
        .route("/js", get(handlers::script::get_script))
        .layer(cors_layer())
        .layer(TraceLayer::new_for_http())
        .with_state(app_state);

    let listener = TcpListener::bind(&config.addr).await?;
    info!(addr = %config.addr, "listening");
    axum::serve(listener, app).await?;

    Ok(())
}

fn init_tracing() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .with_target(true)
        .init();
}
