use std::sync::{Arc, RwLock};

use axum::{Json, extract::State};
use worldstate_parser::{Fissure, WorldState};

#[utoipa::path(
    get,
    path = "/",
    responses(
        (status = 200, description = "Current worldstate", body = WorldState)
    ),
    tag = "worldstate"
)]
pub async fn worldstate(
    State(shared_worldstate): State<Arc<RwLock<WorldState>>>,
) -> Json<WorldState> {
    Json(shared_worldstate.read().unwrap().clone())
}

#[utoipa::path(
    get,
    path = "/fissures",
    responses(
        (status = 200, description = "Current active fissures", body = Vec<Fissure>)
    ),
    tag = "worldstate"
)]
pub async fn fissures(
    State(shared_worldstate): State<Arc<RwLock<WorldState>>>,
) -> Json<Vec<Fissure>> {
    Json(shared_worldstate.read().unwrap().fissures.clone())
}
