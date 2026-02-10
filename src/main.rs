mod handlers;
pub mod worldstate;

use std::sync::{Arc, RwLock};

use axum::{Router, routing::get};
#[cfg(feature = "proxy")]
use reqwest::{ClientBuilder, Proxy, header};
use tracing::level_filters::LevelFilter;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;
use worldstate_parser::{
    cycles::{
        cambion_drift::CambionDriftState,
        cetus::CetusState,
        duviri::DuviriState,
        orb_vallis::OrbVallisState,
    },
    default_data_fetcher::{self, CacheStrategy},
};

use crate::worldstate::{fetch_worldstate_json, get_worldstate, spawn_worldstate_fetcher};

/// OpenAPI Doc
#[derive(OpenApi)]
#[openapi(
    paths(
        handlers::worldstate, handlers::fissures,        
    ),
    components(
        schemas(worldstate_parser::WorldState, DuviriState, CetusState, CambionDriftState, OrbVallisState)
    ),
    tags(
        (name = "worldstate", description = "Warframe Worldstate API")
    )
)]
struct ApiDoc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    default_data_fetcher::fetch_all(CacheStrategy::Basic).await?;

    let client = build_client()?;

    let json = fetch_worldstate_json(&client).await?;

    let shared_worldstate = Arc::new(RwLock::new(get_worldstate(json, &client).await?));

    tokio::spawn(spawn_worldstate_fetcher(
        Arc::clone(&shared_worldstate),
        client,
    ));

    let app = Router::new()
        .route("/", get(handlers::worldstate))
        .route("/fissures", get(handlers::fissures))
        .with_state(shared_worldstate)
        .merge(SwaggerUi::new("/docs").url("/apidoc/openapi.json", ApiDoc::openapi()));

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await?;

    axum::serve(listener, app)
        .with_graceful_shutdown(async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install CTRL+C handler");
        })
        .await?;

    Ok(())
}

fn build_client() -> reqwest::Result<reqwest::Client> {
    let client_builder = reqwest::Client::builder();

    #[cfg(feature = "proxy")]
    let client_builder = apply_proxy(client_builder)?;

    client_builder.build()
}

#[cfg(feature = "proxy")]
fn apply_proxy(builder: ClientBuilder) -> reqwest::Result<ClientBuilder> {
    const USER_AGENT_STR: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/144.0.0.0 Safari/537.36";

    let mut headers = header::HeaderMap::new();
    headers.insert(
        header::USER_AGENT,
        header::HeaderValue::from_static(USER_AGENT_STR),
    );

    headers.insert(
        header::ACCEPT,
        header::HeaderValue::from_static("application/json"),
    );

    let proxy_url = format!(
        "https://{}:{}@{}:{}",
        std::env::var("PROXY_USER").expect("PROXY_USER must be set"),
        std::env::var("PROXY_PASS").expect("PROXY_PASS must be set"),
        std::env::var("PROXY_HOST").expect("PROXY_HOST must be set"),
        std::env::var("PROXY_PORT").expect("PROXY_PORT must be set"),
    );

    Ok(builder
        .proxy(Proxy::all(proxy_url)?)
        .danger_accept_invalid_certs(true)
        .default_headers(headers))
}
