use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct JobAction {
    #[serde(flatten)]
    pub action: Action,
    #[serde(default)]
    pub result_key: Option<String>,
    #[serde(default)]
    pub timeout: Option<u64>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ValueSource {
    Literal(String),
    FromAction(String),
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    Navigate {
        url: String,
    },
    Wait {
        duration: u64,
    },
    WaitForSelector {
        selector: String,
    },
    Click {
        selector: String,
    },
    /// Clicks the visible, enabled element whose exact trimmed text content
    /// matches `text`. Needed for custom-element/shadow-DOM UIs (e.g. Wallapop's
    /// `walla-button`/`walla-list-item`) where no stable CSS selector exists for
    /// individual dropdown options or action buttons.
    ClickText {
        text: String,
    },
    Type {
        selector: String,
        text: ValueSource,
    },
    /// Replaces a native input/textarea's value directly (via its property
    /// setter, then dispatches `input`/`change`), instead of appending
    /// keystrokes like `Type`. Needed to overwrite pre-filled content (e.g.
    /// an AI-generated title) without first selecting existing text.
    Fill {
        selector: String,
        text: ValueSource,
    },
    /// Sets the given file input's files, downloading each `urls` entry first.
    UploadFile {
        selector: String,
        urls: Vec<String>,
    },
    Scroll {
        selector: String,
        direction: ScrollDirection,
    },
    Extract {
        selector: String,
        #[serde(default)]
        attribute: Option<String>,
    },
    /// Captures the page's current URL, e.g. to record the URL a form
    /// submission redirected to.
    ExtractUrl,
}
