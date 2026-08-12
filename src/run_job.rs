use std::path::PathBuf;

use crate::{
    application::ports::ScraperPort,
    domain::{
        entities::Job,
        value_objects::{Action, Cookie, Error, JobAction, JobConfig, ValueSource},
    },
    infrastructure::adapters::ChromiumoxideScraper,
};

mod application;
mod domain;
mod infrastructure;

const UPLOAD_URL: &str = "https://es.wallapop.com/app/catalog/upload/consumer-goods";
const SUMMARY_SELECTOR: &str = "input#summary";
const FILE_INPUT_SELECTOR: &str = "input#dropAreaPreviewInput";
const TITLE_SELECTOR: &str = "input[name='title']";
const DESCRIPTION_SELECTOR: &str = "textarea[name='description']";
const PRICE_SELECTOR: &str = "input[name='price_amount']";
const CATEGORY_MARKER_SELECTOR: &str = "input[name='category_leaf_id']";
const CONTINUE_TEXT: &str = "Continuar";
const PUBLISH_TEXT: &str = "Publicar";

// Mirrors ecore::infrastructure::adapters::platforms::wallapop::platform::WallapopPlatform's
// cookie parsing exactly: Chrome enforces the __Secure-/__Host- cookie prefixes at the
// CDP level (both require `secure`, __Host- additionally forbids `domain`).
fn parse_cookies(raw: &str) -> Vec<Cookie> {
    raw.split(';')
        .filter_map(|pair| {
            let (name, value) = pair.trim().split_once('=')?;
            let name = name.trim();
            if name.is_empty() {
                return None;
            }
            let is_host_only = name.starts_with("__Host-");
            Some(Cookie {
                name: name.to_string(),
                value: value.trim().to_string(),
                domain: if is_host_only {
                    None
                } else {
                    Some(".wallapop.com".to_string())
                },
                path: Some("/".to_string()),
                secure: Some(true),
            })
        })
        .collect()
}

fn literal(text: impl Into<String>) -> ValueSource {
    ValueSource::Literal(text.into())
}

/// Reproduces WallapopPlatform::build_publish_actions exactly, so this can be run
/// from a local IP (instead of the Lambda executor's) to tell apart a genuine bug
/// from an edge/WAF block on AWS-datacenter traffic ("ERROR: The request could not
/// be satisfied" is CloudFront's own error page, not anything Wallapop's app renders).
fn job() -> Result<Job, Error> {
    let cookies_raw = std::env::var("WALLAPOP_COOKIES").map_err(|_| {
        Error::InvalidInput(
            "WALLAPOP_COOKIES must be set (the same \"name=value; name2=value2\" string the \
             connect-platform form sends), e.g.:\n  \
             WALLAPOP_COOKIES='__Secure-next-auth.session-token=...; __Host-next-auth.csrf-token=...; __Secure-next-auth.callback-url=...'"
                .to_string(),
        )
    })?;
    let image_url = std::env::var("WALLAPOP_TEST_IMAGE_URL").map_err(|_| {
        Error::InvalidInput(
            "WALLAPOP_TEST_IMAGE_URL must be set to a publicly reachable jpg/png image URL"
                .to_string(),
        )
    })?;

    let actions = vec![
        JobAction {
            action: Action::Navigate {
                url: UPLOAD_URL.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::WaitForSelector {
                selector: SUMMARY_SELECTOR.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Fill {
                selector: SUMMARY_SELECTOR.to_string(),
                text: literal("Test product - ignore"),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ClickText {
                text: CONTINUE_TEXT.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::WaitForSelector {
                selector: FILE_INPUT_SELECTOR.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::UploadFile {
                selector: FILE_INPUT_SELECTOR.to_string(),
                urls: vec![image_url],
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Wait { duration: 1500 },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ClickText {
                text: CONTINUE_TEXT.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::WaitForSelector {
                selector: CATEGORY_MARKER_SELECTOR.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Wait { duration: 3000 },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Fill {
                selector: TITLE_SELECTOR.to_string(),
                text: literal("Test product - ignore"),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Fill {
                selector: DESCRIPTION_SELECTOR.to_string(),
                text: literal("Test product - ignore"),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ClickText {
                text: "Estado*".to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Wait { duration: 300 },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ClickText {
                text: "Nuevo".to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Fill {
                selector: PRICE_SELECTOR.to_string(),
                text: literal("1"),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ClickText {
                text: PUBLISH_TEXT.to_string(),
            },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::Wait { duration: 2500 },
            result_key: None,
            timeout: None,
        },
        JobAction {
            action: Action::ExtractUrl,
            result_key: Some("final_url".to_string()),
            timeout: None,
        },
    ];

    let config = JobConfig {
        cookies: Some(parse_cookies(&cookies_raw)),
        user_agent: None,
        proxy: std::env::var("WALLAPOP_TEST_PROXY").ok(),
    };

    Job::new(actions, Some(config))
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
