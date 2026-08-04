use std::sync::Arc;

use axum::{
    routing::{get, post},
    Router,
};
use lambda_http::{run, Error};

use crate::infrastructure::{bootstrap::setup, http::job_routes};

mod application;
mod domain;
mod infrastructure;

async fn get_router() -> Router {
    let state = setup().await;
    Router::new()
        .route("/jobs", post(job_routes::create_job))
        .route("/jobs/{job_id}", get(job_routes::get_job))
        .with_state(Arc::new(state))
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_ansi(false)
        .without_time()
        .init();
    let router = get_router().await;
    run(router).await
}
