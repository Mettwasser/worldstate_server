use std::{
    path::Path,
    sync::{Arc, RwLock},
    time::Duration,
};

use tokio::time::sleep;
use worldstate_parser::{
    WorldState,
    WorldstateError,
    default_context_provider::{DefaultContextProvider, PathContext},
};

pub async fn fetch_worldstate_json(client: &reqwest::Client) -> Result<String, reqwest::Error> {
    client
        .get("https://api.warframe.com/cdn/worldState.php")
        .send()
        .await?
        .text()
        .await
}

pub async fn get_worldstate(
    json: String,
    client: &reqwest::Client,
) -> Result<WorldState, WorldstateError> {
    WorldState::from_str(
        &json,
        DefaultContextProvider(
            PathContext {
                data_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("data/"),
                drops_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("drops/"),
                assets_dir: &Path::new(env!("CARGO_MANIFEST_DIR")).join("assets/"),
            },
            client,
        ),
    )
    .await
}

pub async fn spawn_worldstate_fetcher(
    shared_worldstate: Arc<RwLock<WorldState>>,
    client: reqwest::Client,
) {
    loop {
        tracing::info!("fetcher: sleeping 5 min");
        sleep(Duration::from_mins(5)).await;

        let Ok(json) = fetch_worldstate_json(&client).await else {
            continue;
        };

        let Ok(worldstate) = get_worldstate(json, &client).await else {
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
