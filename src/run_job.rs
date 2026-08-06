use std::path::PathBuf;

use crate::{
    application::ports::ScraperPort,
    domain::{
        entities::Job,
        value_objects::{Action, Error, JobAction},
    },
    infrastructure::adapters::ChromiumoxideScraper,
};

mod application;
mod domain;
mod infrastructure;

fn job() -> Result<Job, Error> {
    let actions = vec![
        JobAction {
            action: Action::Navigate {
                url: "https://example.com".to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Extract {
                selector: "h1".to_string(),
                attribute: None,
            },
            result_key: Some("title".to_string()),
            timeout: None,
        },
    ];

    Job::new(actions, None)
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .init();

    let job = job()?;

    let chrome_executable_path = std::env::var("CHROME_EXECUTABLE_PATH")
        .ok()
        .map(PathBuf::from);
    let scraper = ChromiumoxideScraper::new(chrome_executable_path, false);

    let results = scraper.execute(&job).await?;
    println!("{results:#?}");

    Ok(())
}
