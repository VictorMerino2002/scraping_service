pub struct JobAction {
    pub action: Action,
    pub result_key: Option<String>,
    pub timeout: Option<u64>,
}

pub enum ValueSource {
    Literal(String),
    FromAction(String),
}

pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

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
        attribute: Option<String>,
    },
}
