use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{Json, Router, extract::State, routing::get};
use reqwest::{Proxy, header};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use worldstate_parser::{
    WorldState,
    WorldstateError,
    default_context_provider::{DefaultContextProvider, PathContext},
    default_data_fetcher::{self, CacheStrategy},
};

async fn fetch_worldstate_json(client: &reqwest::Client) -> Result<String, reqwest::Error> {
    client
        .get("https://api.warframe.com/cdn/worldState.php")
        .send()
        .await?
        .text()
        .await
}

async fn get_worldstate(json: String) -> Result<WorldState, WorldstateError> {
    WorldState::from_str(
        &json,
        DefaultContextProvider(PathContext {
            data_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("data/"),
            drops_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("drops/"),
            assets_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/"),
        }),
    )
    .await
}

async fn spawn_worldstate_fetcher(
    shared_worldstate: Arc<RwLock<WorldState>>,
    client: reqwest::Client,
) {
    loop {
        tracing::info!("fetcher: sleeping 5 min");
        sleep(Duration::from_mins(5)).await;

        let Ok(json) = fetch_worldstate_json(&client).await else {
            continue;
        };

        let Ok(worldstate) = get_worldstate(json).await else {
            continue;
        };

        let mut old_worldstate = shared_worldstate.write().unwrap();
        tracing::info!("fetcher: Fetched new worldstate.");

        tracing::info!(
            "Fissures changed? {}",
            worldstate.fissures == old_worldstate.fissures
        );

        if *old_worldstate != worldstate {
            *old_worldstate = worldstate;
            tracing::info!("fetcher: Changes found. Made available in shared state.");
        } else {
            tracing::info!("fetcher: No changes in the worldstate. Skipping.");
        }
    }
}

async fn worldstate_handler(
    State(shared_worldstate): State<Arc<RwLock<WorldState>>>,
) -> Json<WorldState> {
    Json(shared_worldstate.read().unwrap().clone())
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install CTRL+C handler");
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    default_data_fetcher::fetch_all(CacheStrategy::Basic).await?;

    let proxy_url = format!(
        "http://{}:{}@{}:{}",
        std::env::var("PROXY_USER").expect("PROXY_USER must be set"),
        std::env::var("PROXY_PASS").expect("PROXY_PASS must be set"),
        std::env::var("PROXY_HOST").expect("PROXY_HOST must be set"),
        std::env::var("PROXY_PORT").expect("PROXY_PORT must be set"),
    );

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
    );
    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );

    let client = reqwest::Client::builder()
        .proxy(Proxy::all(proxy_url)?)
        .default_headers(headers)
        .build()?;

    let json = fetch_worldstate_json(&client).await?;

    let shared_worldstate = Arc::new(RwLock::new(get_worldstate(json).await?));

    tokio::spawn(spawn_worldstate_fetcher(
        Arc::clone(&shared_worldstate),
        client,
    ));

    let app = Router::new()
        .route("/worldstate", get(worldstate_handler))
        .with_state(shared_worldstate);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}
