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
    Type {
        selector: String,
        text: ValueSource,
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
}
