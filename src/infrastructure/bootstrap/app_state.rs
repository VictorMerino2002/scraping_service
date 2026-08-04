use std::path::PathBuf;
use std::sync::Arc;

use aws_sdk_dynamodb::Client as DynamoDbClient;
use aws_sdk_s3::Client as S3Client;

use crate::{
    application::ports::{ActionSerializer, JobRepositoryPort, ScraperPort},
    infrastructure::adapters::{
        ChromiumoxideScraper, DynamoS3JobRepository, JsonActionSerializer,
        DEFAULT_S3_OFFLOAD_THRESHOLD_BYTES,
    },
};

pub struct AppState {
    pub job_repository: Arc<dyn JobRepositoryPort>,
    pub scraper: Arc<dyn ScraperPort>,
    pub action_serializer: Arc<dyn ActionSerializer<Input = String> + Send + Sync>,
}

pub async fn setup() -> AppState {
    let aws_config = aws_config::load_from_env().await;
    let dynamo_client = DynamoDbClient::new(&aws_config);
    let s3_client = S3Client::new(&aws_config);

    let table_name = std::env::var("JOBS_TABLE_NAME")
        .expect("JOBS_TABLE_NAME must be set in environment variables");
    let bucket_name = std::env::var("JOBS_BUCKET_NAME")
        .expect("JOBS_BUCKET_NAME must be set in environment variables");
    let chrome_executable_path = std::env::var("CHROME_EXECUTABLE_PATH").ok().map(PathBuf::from);

    let job_repository = Arc::new(DynamoS3JobRepository::new(
        dynamo_client,
        s3_client,
        JsonActionSerializer,
        table_name,
        bucket_name,
        DEFAULT_S3_OFFLOAD_THRESHOLD_BYTES,
    ));

    let scraper = Arc::new(ChromiumoxideScraper::new(chrome_executable_path));
    let action_serializer = Arc::new(JsonActionSerializer);

    AppState {
        job_repository,
        scraper,
        action_serializer,
    }
}
