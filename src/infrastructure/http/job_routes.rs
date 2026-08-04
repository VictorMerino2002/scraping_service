use std::sync::Arc;

use axum::{
    extract::{Path, State},
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    application::use_cases::{CreateJobUseCase, GetJobUseCase},
    domain::{
        entities::Job,
        value_objects::{ApiResponse, JobAction, JobConfig, JobStatus},
    },
    infrastructure::{bootstrap::AppState, http::auth_header_extractor::AuthHeader},
};

#[derive(Deserialize)]
pub struct CreateJobRequest {
    actions: serde_json::Value,
    config: Option<JobConfig>,
}

#[derive(Serialize)]
pub struct CreateJobResponse {
    id: Uuid,
}

#[derive(Serialize)]
pub struct JobResponse {
    id: Uuid,
    status: JobStatus,
    actions: Vec<JobAction>,
    config: Option<JobConfig>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

impl From<&Job> for JobResponse {
    fn from(job: &Job) -> Self {
        Self {
            id: job.id(),
            status: job.status(),
            actions: job.actions().to_vec(),
            config: job.config().cloned(),
            error: job.error().map(str::to_string),
            created_at: job.created_at(),
            started_at: job.started_at(),
            finished_at: job.finished_at(),
        }
    }
}

pub async fn create_job(
    State(state): State<Arc<AppState>>,
    _: AuthHeader,
    Json(payload): Json<CreateJobRequest>,
) -> impl IntoResponse {
    let use_case = CreateJobUseCase {
        action_serializer: state.action_serializer.clone(),
        job_repository: state.job_repository.clone(),
    };

    let actions_json = serde_json::to_string(&payload.actions).unwrap_or_default();
    let result = use_case
        .execute(actions_json, payload.config)
        .await
        .map(|id| CreateJobResponse { id });

    ApiResponse::default(result)
}

pub async fn get_job(
    State(state): State<Arc<AppState>>,
    _: AuthHeader,
    Path(job_id): Path<Uuid>,
) -> impl IntoResponse {
    let use_case = GetJobUseCase {
        job_repository: state.job_repository.clone(),
    };

    let result = use_case
        .execute(&job_id)
        .await
        .map(|job| JobResponse::from(&job));

    ApiResponse::default(result)
}
