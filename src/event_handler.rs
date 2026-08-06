use std::sync::Arc;

use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use serde::Deserialize;
use uuid::Uuid;

use crate::{application::use_cases::ExecuteJobUseCase, infrastructure::bootstrap::setup};

mod application;
mod domain;
mod infrastructure;

#[derive(Deserialize)]
struct ExecuteJobEvent {
    job_id: Uuid,
}

async fn function_handler(
    event: LambdaEvent<ExecuteJobEvent>,
    use_case: Arc<ExecuteJobUseCase>,
) -> Result<(), String> {
    use_case
        .execute(&event.payload.job_id)
        .await
        .map_err(|error| error.to_string())
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .with_ansi(false)
        .without_time()
        .init();

    let state = setup().await;
    let use_case = Arc::new(ExecuteJobUseCase {
        scraper: state.scraper,
        job_repository: state.job_repository,
    });

    run(service_fn(move |event: LambdaEvent<ExecuteJobEvent>| {
        let use_case = Arc::clone(&use_case);
        async move { function_handler(event, use_case).await }
    }))
    .await
}
