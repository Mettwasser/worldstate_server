use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use axum::{Json, Router, extract::State, routing::get};
use tokio::time::sleep;
use tracing::level_filters::LevelFilter;
use worldstate_parser::{
    default_context_provider::DefaultContextProvider,
    default_context_provider::PathContext,
    worldstate::{self, WorldState, WorldstateError},
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
    worldstate::from_str(
        &json,
        DefaultContextProvider,
        PathContext {
            data_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("data/"),
            drops_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("drops/"),
            assets_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/"),
        },
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
            sleep(Duration::from_mins(5)).await;
            continue;
        };

        let Ok(worldstate) = get_worldstate(json).await else {
            sleep(Duration::from_mins(5)).await;
            continue;
        };

        let mut old_worldstate = shared_worldstate.write().unwrap();
        tracing::info!("fetcher: Fetched new worldstate.");

        if old_worldstate.fissures != worldstate.fissures {
            tracing::info!(
                "Old fissures: {:?}",
                old_worldstate
                    .fissures
                    .iter()
                    .map(|f| format!(
                        "{} | {}",
                        f.node.as_ref().unwrap().name,
                        f.node.as_ref().unwrap().planet
                    ))
                    .collect::<Vec<_>>()
            );

            tracing::info!(
                "New fissures: {:?}",
                worldstate
                    .fissures
                    .iter()
                    .map(|f| format!(
                        "{} | {}",
                        f.node.as_ref().unwrap().name,
                        f.node.as_ref().unwrap().planet
                    ))
                    .collect::<Vec<_>>()
            );
        }
        tracing::info!(
            "Fissures match: {}",
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

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_max_level(LevelFilter::INFO)
        .init();

    let client = reqwest::Client::new();

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
    axum::serve(listener, app).await?;

    Ok(())
}
