use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use async_trait::async_trait;
use chromiumoxide::cdp::browser_protocol::dom::SetFileInputFilesParams;
use chromiumoxide::cdp::browser_protocol::network::CookieParam;
use chromiumoxide::layout::Point;
use chromiumoxide::{Browser, BrowserConfig, Element, Page};
use futures::StreamExt;
use serde::Deserialize;

use crate::application::ports::ScraperPort;
use crate::domain::entities::Job;
use crate::domain::value_objects::{
    Action, Error, JobAction, JobConfig, ScrollDirection, ValueSource,
};

const DEFAULT_ACTION_TIMEOUT_MS: u64 = 30_000;
const WAIT_FOR_SELECTOR_POLL_INTERVAL_MS: u64 = 100;
const CLICK_TEXT_POLL_INTERVAL_MS: u64 = 150;
const SCROLL_STEP_PX: i64 = 500;

#[derive(Deserialize)]
struct ViewportPoint {
    x: f64,
    y: f64,
}

const SERVERLESS_CHROME_ARGS: [&str; 3] =
    ["--no-sandbox", "--disable-gpu", "--disable-dev-shm-usage"];

pub struct ChromiumoxideScraper {
    chrome_executable_path: Option<PathBuf>,
    headless: bool,
}

impl ChromiumoxideScraper {
    pub fn new(chrome_executable_path: Option<PathBuf>, headless: bool) -> Self {
        Self {
            chrome_executable_path,
            headless,
        }
    }
}

#[async_trait]
impl ScraperPort for ChromiumoxideScraper {
    async fn execute(&self, job: &Job) -> Result<HashMap<String, String>, Error> {
        let mut builder = BrowserConfig::builder().args(SERVERLESS_CHROME_ARGS);

        if !self.headless {
            builder = builder.with_head();
        }

        if let Some(path) = &self.chrome_executable_path {
            builder = builder.chrome_executable(path);
        }

        if let Some(proxy) = job.config().and_then(|config| config.proxy.as_ref()) {
            builder = builder.arg(format!("--proxy-server={proxy}"));
        }

        let browser_config = builder.build().map_err(Error::Unknown)?;

        let (mut browser, mut handler) = Browser::launch(browser_config)
            .await
            .map_err(|error| Error::Unknown(error.to_string()))?;

        let handler_task = tokio::spawn(async move { while handler.next().await.is_some() {} });

        let result = Self::run_job(&browser, job).await;

        let _ = browser.close().await;
        handler_task.abort();

        result
    }
}

impl ChromiumoxideScraper {
    async fn run_job(browser: &Browser, job: &Job) -> Result<HashMap<String, String>, Error> {
        let page = browser
            .new_page("about:blank")
            .await
            .map_err(|error| Error::NetworkError(error.to_string()))?;

        if let Some(config) = job.config() {
            Self::apply_config(&page, config).await?;
        }

        let mut results = HashMap::new();

        for job_action in job.actions() {
            let timeout =
                Duration::from_millis(job_action.timeout.unwrap_or(DEFAULT_ACTION_TIMEOUT_MS));

            let value =
                tokio::time::timeout(timeout, Self::execute_action(&page, job_action, &results))
                    .await
                    .map_err(|_| Error::NetworkError("action timed out".to_string()))??;

            if let (Some(key), Some(value)) = (&job_action.result_key, value) {
                results.insert(key.clone(), value);
            }
        }

        Ok(results)
    }

    async fn apply_config(page: &Page, config: &JobConfig) -> Result<(), Error> {
        if let Some(user_agent) = &config.user_agent {
            page.set_user_agent(user_agent.as_str())
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;
        }

        if let Some(cookies) = &config.cookies {
            let cookie_params: Vec<CookieParam> = cookies
                .iter()
                .cloned()
                .map(|cookie| CookieParam {
                    domain: cookie.domain,
                    path: cookie.path,
                    ..CookieParam::new(cookie.name, cookie.value)
                })
                .collect();

            page.set_cookies(cookie_params)
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;
        }

        Ok(())
    }

    async fn execute_action(
        page: &Page,
        job_action: &JobAction,
        results: &HashMap<String, String>,
    ) -> Result<Option<String>, Error> {
        match &job_action.action {
            Action::Navigate { url } => {
                page.goto(url.as_str())
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::Wait { duration } => {
                tokio::time::sleep(Duration::from_millis(*duration)).await;
                Ok(None)
            }
            Action::WaitForSelector { selector } => {
                Self::wait_for_selector(page, selector.as_str()).await?;
                Ok(None)
            }
            Action::Click { selector } => {
                let element = Self::wait_for_selector(page, selector.as_str()).await?;
                element
                    .click()
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::ClickText { text } => {
                let point = Self::wait_for_visible_text(page, text.as_str()).await?;
                page.click(point)
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::Type { selector, text } => {
                let value = Self::resolve_value_source(text, results)?;
                let element = Self::wait_for_selector(page, selector.as_str()).await?;
                element
                    .type_str(value)
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::Fill { selector, text } => {
                let value = Self::resolve_value_source(text, results)?;
                let element = Self::wait_for_selector(page, selector.as_str()).await?;
                element
                    .call_js_fn(Self::build_fill_script(&value), true)
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::UploadFile { selector, urls } => {
                let element = Self::wait_for_selector(page, selector.as_str()).await?;
                let paths = Self::download_files(urls).await?;
                let params = SetFileInputFilesParams::builder()
                    .files(paths.iter().map(|p| p.to_string_lossy().into_owned()))
                    .backend_node_id(element.backend_node_id)
                    .build()
                    .map_err(Error::Unknown)?;
                page.execute(params)
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::Scroll {
                selector,
                direction,
            } => {
                let script = Self::build_scroll_script(selector.as_str(), direction);
                page.evaluate_expression(script)
                    .await
                    .map_err(|error| Error::NetworkError(error.to_string()))?;
                Ok(None)
            }
            Action::Extract {
                selector,
                attribute,
            } => {
                let element = Self::wait_for_selector(page, selector.as_str()).await?;
                let value = match attribute {
                    Some(attribute) => element
                        .attribute(attribute)
                        .await
                        .map_err(|error| Error::NetworkError(error.to_string()))?,
                    None => element
                        .inner_text()
                        .await
                        .map_err(|error| Error::NetworkError(error.to_string()))?,
                };
                Ok(value)
            }
            Action::ExtractUrl => page
                .url()
                .await
                .map_err(|error| Error::NetworkError(error.to_string())),
        }
    }

    async fn wait_for_selector(page: &Page, selector: &str) -> Result<Element, Error> {
        loop {
            match page.find_element(selector).await {
                Ok(element) => return Ok(element),
                Err(_) => {
                    tokio::time::sleep(Duration::from_millis(WAIT_FOR_SELECTOR_POLL_INTERVAL_MS))
                        .await;
                }
            }
        }
    }

    /// Polls until an enabled, unobstructed element with the exact trimmed
    /// text content `text` is on screen, and returns its center point in
    /// viewport coordinates. Searches through shadow roots (`querySelector`
    /// alone can't see inside them), since Wallapop's UI renders its
    /// clickable labels (dropdown options, action buttons) that way with no
    /// stable per-item CSS selector.
    async fn wait_for_visible_text(page: &Page, text: &str) -> Result<Point, Error> {
        let script = Self::build_click_text_script(text);
        loop {
            let result = page
                .evaluate_expression(script.clone())
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;

            if let Ok(Some(point)) = result.into_value::<Option<ViewportPoint>>() {
                return Ok(Point::new(point.x, point.y));
            }

            tokio::time::sleep(Duration::from_millis(CLICK_TEXT_POLL_INTERVAL_MS)).await;
        }
    }

    async fn download_files(urls: &[String]) -> Result<Vec<PathBuf>, Error> {
        let client = reqwest::Client::new();
        let batch_id = uuid::Uuid::new_v4();
        let mut paths = Vec::with_capacity(urls.len());

        for (index, url) in urls.iter().enumerate() {
            let response = client
                .get(url)
                .send()
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;

            if !response.status().is_success() {
                return Err(Error::NetworkError(format!(
                    "failed to download '{url}': HTTP {}",
                    response.status()
                )));
            }

            let extension = std::path::Path::new(url.split('?').next().unwrap_or(url))
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("jpg");
            let path =
                std::env::temp_dir().join(format!("scrape-upload-{batch_id}-{index}.{extension}"));

            let bytes = response
                .bytes()
                .await
                .map_err(|error| Error::NetworkError(error.to_string()))?;
            tokio::fs::write(&path, &bytes)
                .await
                .map_err(|error| Error::Unknown(error.to_string()))?;

            paths.push(path);
        }

        Ok(paths)
    }

    fn resolve_value_source(
        value: &ValueSource,
        results: &HashMap<String, String>,
    ) -> Result<String, Error> {
        match value {
            ValueSource::Literal(value) => Ok(value.clone()),
            ValueSource::FromAction(key) => results
                .get(key)
                .cloned()
                .ok_or_else(|| Error::InvalidInput(format!("no result found for key '{key}'"))),
        }
    }

    fn build_scroll_script(selector: &str, direction: &ScrollDirection) -> String {
        // Serialize the selector as a JSON string to get a properly escaped
        // JS string literal, avoiding naive interpolation / script injection.
        let selector_literal =
            serde_json::to_string(selector).unwrap_or_else(|_| "\"\"".to_string());
        let (dx, dy) = match direction {
            ScrollDirection::Up => (0, -SCROLL_STEP_PX),
            ScrollDirection::Down => (0, SCROLL_STEP_PX),
            ScrollDirection::Left => (-SCROLL_STEP_PX, 0),
            ScrollDirection::Right => (SCROLL_STEP_PX, 0),
        };

        format!("document.querySelector({selector_literal}).scrollBy({dx}, {dy})")
    }

    fn build_fill_script(text: &str) -> String {
        // Serialize the text as a JSON string to get a properly escaped JS
        // string literal, avoiding naive interpolation / script injection.
        let text_literal = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());

        format!(
            r#"function() {{
                const setter = Object.getOwnPropertyDescriptor(Object.getPrototypeOf(this), 'value').set;
                setter.call(this, {text_literal});
                this.dispatchEvent(new Event('input', {{bubbles: true}}));
                this.dispatchEvent(new Event('change', {{bubbles: true}}));
            }}"#
        )
    }

    fn build_click_text_script(text: &str) -> String {
        // Serialize the text as a JSON string to get a properly escaped JS
        // string literal, avoiding naive interpolation / script injection.
        let text_literal = serde_json::to_string(text).unwrap_or_else(|_| "\"\"".to_string());

        format!(
            r#"(function() {{
                const target = {text_literal};

                function deepAll(root, out) {{
                    root.querySelectorAll('*').forEach((el) => {{
                        out.push(el);
                        if (el.shadowRoot) deepAll(el.shadowRoot, out);
                    }});
                }}

                function isDisabled(el) {{
                    let node = el;
                    for (let hops = 0; node && hops < 8; hops++) {{
                        if (node.disabled === true) return true;
                        if (node.getAttribute && (
                            node.getAttribute('aria-disabled') === 'true' ||
                            node.getAttribute('disabled') !== null
                        )) return true;
                        node = node.parentElement || (node.getRootNode() && node.getRootNode().host);
                    }}
                    return false;
                }}

                // `document.elementFromPoint` stops at the nearest shadow host
                // instead of piercing into nested shadow roots, so the "is this
                // on top" check below walks the same light-DOM/shadow-host chain
                // rather than relying on `Node.contains`, which also can't cross
                // shadow boundaries.
                function ancestorChain(el) {{
                    const chain = [];
                    let node = el;
                    for (let hops = 0; node && hops < 12; hops++) {{
                        chain.push(node);
                        node = node.parentElement || (node.getRootNode() && node.getRootNode().host);
                    }}
                    return chain;
                }}

                const all = [];
                deepAll(document, all);
                const matches = all.filter((el) =>
                    el.childElementCount === 0 &&
                    el.textContent &&
                    el.textContent.trim() === target
                );

                for (const el of matches) {{
                    if (isDisabled(el)) continue;

                    el.scrollIntoView({{block: 'center', inline: 'center'}});
                    const rect = el.getBoundingClientRect();
                    if (rect.width === 0 || rect.height === 0) continue;

                    const cx = rect.left + rect.width / 2;
                    const cy = rect.top + rect.height / 2;
                    const top = document.elementFromPoint(cx, cy);
                    const chain = ancestorChain(el);
                    if (top && (chain.includes(top) || (top.contains && top.contains(el)))) {{
                        return {{x: cx, y: cy}};
                    }}
                }}

                return null;
            }})()"#
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escapes_the_text_in_the_fill_script() {
        let script = ChromiumoxideScraper::build_fill_script(r#"a "quoted" value"#);

        assert!(script.contains(r#"setter.call(this, "a \"quoted\" value");"#));
    }

    #[test]
    fn escapes_the_text_in_the_click_text_script() {
        let script = ChromiumoxideScraper::build_click_text_script(r#"Sí, "eliminarlo""#);

        assert!(script.contains(r#"const target = "Sí, \"eliminarlo\"";"#));
    }

    #[test]
    fn resolves_a_literal_value() {
        let results = HashMap::new();
        let value = ChromiumoxideScraper::resolve_value_source(
            &ValueSource::Literal("hello".to_string()),
            &results,
        )
        .unwrap();

        assert_eq!(value, "hello");
    }

    #[test]
    fn resolves_a_value_from_a_previous_action() {
        let mut results = HashMap::new();
        results.insert("title".to_string(), "rust".to_string());

        let value = ChromiumoxideScraper::resolve_value_source(
            &ValueSource::FromAction("title".to_string()),
            &results,
        )
        .unwrap();

        assert_eq!(value, "rust");
    }

    #[test]
    fn fails_when_the_referenced_key_is_missing() {
        let results = HashMap::new();
        let result = ChromiumoxideScraper::resolve_value_source(
            &ValueSource::FromAction("missing".to_string()),
            &results,
        );

        assert!(result.is_err());
    }

    #[test]
    fn escapes_the_selector_in_the_scroll_script() {
        let script = ChromiumoxideScraper::build_scroll_script(
            r#"div[data-test="a\"b"]"#,
            &ScrollDirection::Down,
        );

        assert!(script.contains(r#"document.querySelector("div[data-test=\"a\\\"b\"]")"#));
        assert!(script.contains("scrollBy(0, 500)"));
    }

    #[test]
    fn scroll_directions_map_to_the_expected_deltas() {
        assert!(
            ChromiumoxideScraper::build_scroll_script("sel", &ScrollDirection::Up)
                .contains("scrollBy(0, -500)")
        );
        assert!(
            ChromiumoxideScraper::build_scroll_script("sel", &ScrollDirection::Left)
                .contains("scrollBy(-500, 0)")
        );
        assert!(
            ChromiumoxideScraper::build_scroll_script("sel", &ScrollDirection::Right)
                .contains("scrollBy(500, 0)")
        );
    }
}
