use std::collections::HashMap;

use async_trait::async_trait;
use aws_sdk_dynamodb::types::AttributeValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::application::ports::{ActionSerializer, JobRepositoryPort};
use crate::domain::entities::Job;
use crate::domain::value_objects::{Error, JobConfig, JobStatus};

pub const DEFAULT_S3_OFFLOAD_THRESHOLD_BYTES: usize = 300_000;

pub struct DynamoS3JobRepository<S: ActionSerializer<Input = String>> {
    dynamo_client: aws_sdk_dynamodb::Client,
    s3_client: aws_sdk_s3::Client,
    action_serializer: S,
    table_name: String,
    bucket_name: String,
    s3_offload_threshold_bytes: usize,
}

impl<S: ActionSerializer<Input = String>> DynamoS3JobRepository<S> {
    pub fn new(
        dynamo_client: aws_sdk_dynamodb::Client,
        s3_client: aws_sdk_s3::Client,
        action_serializer: S,
        table_name: String,
        bucket_name: String,
        s3_offload_threshold_bytes: usize,
    ) -> Self {
        Self {
            dynamo_client,
            s3_client,
            action_serializer,
            table_name,
            bucket_name,
            s3_offload_threshold_bytes,
        }
    }

    fn actions_s3_key(job_id: &Uuid) -> String {
        format!("jobs/{job_id}/actions.json")
    }

    fn results_s3_key(job_id: &Uuid) -> String {
        format!("jobs/{job_id}/results.json")
    }

    /// Stores `payload` inline if it fits under the offload threshold, otherwise
    /// uploads it to S3 under `s3_key` and returns the key instead. Shared by both
    /// the `actions` and `results` fields, which follow the same inline-or-S3 shape.
    async fn store_payload(
        &self,
        s3_key: String,
        payload: String,
    ) -> Result<(Option<String>, Option<String>), Error> {
        if payload.len() > self.s3_offload_threshold_bytes {
            self.s3_client
                .put_object()
                .bucket(&self.bucket_name)
                .key(&s3_key)
                .content_type("application/json")
                .body(payload.into_bytes().into())
                .send()
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;

            Ok((None, Some(s3_key)))
        } else {
            Ok((Some(payload), None))
        }
    }

    /// Reverse of `store_payload`: resolves the inline value or fetches it from S3.
    async fn load_payload(
        &self,
        inline: Option<String>,
        s3_key: Option<String>,
    ) -> Result<Option<String>, Error> {
        match (inline, s3_key) {
            (Some(payload), _) => Ok(Some(payload)),
            (None, Some(key)) => {
                let output = self
                    .s3_client
                    .get_object()
                    .bucket(&self.bucket_name)
                    .key(&key)
                    .send()
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;

                let bytes = output
                    .body
                    .collect()
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?
                    .into_bytes();

                let payload = String::from_utf8(bytes.to_vec())
                    .map_err(|error| Error::Unknown(error.to_string()))?;

                Ok(Some(payload))
            }
            (None, None) => Ok(None),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
struct JobRecord {
    id: Uuid,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actions_inline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    actions_s3_key: Option<String>,
    status: JobStatus,
    config: Option<JobConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    results_inline: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    results_s3_key: Option<String>,
    error: Option<String>,
    created_at: DateTime<Utc>,
    started_at: Option<DateTime<Utc>>,
    finished_at: Option<DateTime<Utc>>,
}

#[async_trait]
impl<S: ActionSerializer<Input = String>> JobRepositoryPort for DynamoS3JobRepository<S> {
    async fn save(&self, job: &Job) -> Result<(), Error> {
        let actions_json = self.action_serializer.to(job.actions())?;
        let (actions_inline, actions_s3_key) = self
            .store_payload(Self::actions_s3_key(&job.id()), actions_json)
            .await?;

        let (results_inline, results_s3_key) = match job.results() {
            Some(results) => {
                let results_json = serde_json::to_string(results)
                    .map_err(|error| Error::Unknown(error.to_string()))?;
                self.store_payload(Self::results_s3_key(&job.id()), results_json)
                    .await?
            }
            None => (None, None),
        };

        let record = JobRecord {
            id: job.id(),
            actions_inline,
            actions_s3_key,
            status: job.status(),
            config: job.config().cloned(),
            results_inline,
            results_s3_key,
            error: job.error().map(str::to_string),
            created_at: job.created_at(),
            started_at: job.started_at(),
            finished_at: job.finished_at(),
        };

        let item = serde_dynamo::to_item(&record)
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        self.dynamo_client
            .put_item()
            .table_name(&self.table_name)
            .set_item(Some(item))
            .send()
            .await
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        Ok(())
    }

    async fn get_by_id(&self, id: &Uuid) -> Result<Option<Job>, Error> {
        let output = self
            .dynamo_client
            .get_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        let Some(item) = output.item else {
            return Ok(None);
        };

        let record: JobRecord = serde_dynamo::from_item(item)
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        let actions_json = self
            .load_payload(record.actions_inline, record.actions_s3_key)
            .await?
            .ok_or_else(|| {
                Error::DatabaseError(
                    "job record has neither inline actions nor an S3 key".to_string(),
                )
            })?;
        let actions = self.action_serializer.from(actions_json)?;

        let results_json = self
            .load_payload(record.results_inline, record.results_s3_key)
            .await?;
        let results = results_json
            .map(|json| {
                serde_json::from_str::<HashMap<String, String>>(&json)
                    .map_err(|error| Error::Unknown(error.to_string()))
            })
            .transpose()?;

        Ok(Some(Job::from_persisted(
            record.id,
            actions,
            record.status,
            record.config,
            results,
            record.error,
            record.created_at,
            record.started_at,
            record.finished_at,
        )))
    }

    async fn delete(&self, id: &Uuid) -> Result<(), Error> {
        let output = self
            .dynamo_client
            .get_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        if let Some(item) = output.item {
            let record: JobRecord = serde_dynamo::from_item(item)
                .map_err(|error| Error::DatabaseError(error.to_string()))?;

            for key in [record.actions_s3_key, record.results_s3_key]
                .into_iter()
                .flatten()
            {
                self.s3_client
                    .delete_object()
                    .bucket(&self.bucket_name)
                    .key(&key)
                    .send()
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
            }
        }

        self.dynamo_client
            .delete_item()
            .table_name(&self.table_name)
            .key("id", AttributeValue::S(id.to_string()))
            .send()
            .await
            .map_err(|error| Error::DatabaseError(error.to_string()))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_a_record_with_inline_actions() {
        let record = JobRecord {
            id: Uuid::new_v4(),
            actions_inline: Some(
                r##"[{"action":"navigate","url":"https://example.com"}]"##.to_string(),
            ),
            actions_s3_key: None,
            status: JobStatus::Pending,
            config: None,
            results_inline: None,
            results_s3_key: None,
            error: None,
            created_at: Utc::now(),
            started_at: None,
            finished_at: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("actions_s3_key"));
        assert!(!json.contains("results_inline"));
        assert!(!json.contains("results_s3_key"));

        let round_tripped: JobRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, record);
    }

    #[test]
    fn round_trips_a_record_with_actions_offloaded_to_s3() {
        let record = JobRecord {
            id: Uuid::new_v4(),
            actions_inline: None,
            actions_s3_key: Some("jobs/some-id/actions.json".to_string()),
            status: JobStatus::InProgress,
            config: None,
            results_inline: Some(r##"{"title":"rust"}"##.to_string()),
            results_s3_key: None,
            error: None,
            created_at: Utc::now(),
            started_at: Some(Utc::now()),
            finished_at: None,
        };

        let json = serde_json::to_string(&record).unwrap();
        assert!(!json.contains("actions_inline"));

        let round_tripped: JobRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, record);
    }
}
