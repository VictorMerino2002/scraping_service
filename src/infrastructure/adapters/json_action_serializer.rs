use crate::application::ports::ActionSerializer;
use crate::domain::value_objects::{Error, JobAction};

pub struct JsonActionSerializer;

impl ActionSerializer for JsonActionSerializer {
    type Input = String;

    fn from(&self, input: String) -> Result<Vec<JobAction>, Error> {
        serde_json::from_str(&input).map_err(|error| Error::InvalidInput(error.to_string()))
    }

    fn to(&self, actions: &[JobAction]) -> Result<String, Error> {
        serde_json::to_string(actions).map_err(|error| Error::Unknown(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_a_list_of_actions() {
        let input = r##"[
            {
                "action": "navigate",
                "url": "https://example.com",
                "result_key": null,
                "timeout": 5000
            },
            {
                "action": "type",
                "selector": "#search",
                "text": {"literal": "rust"}
            },
            {
                "action": "extract",
                "selector": ".title",
                "result_key": "title"
            }
        ]"##
        .to_string();

        let actions = JsonActionSerializer.from(input).unwrap();

        assert_eq!(actions.len(), 3);
    }

    #[test]
    fn fails_on_invalid_json() {
        let result = JsonActionSerializer.from("not json".to_string());

        assert!(result.is_err());
    }

    #[test]
    fn round_trips_through_to_and_from() {
        let input = r##"[{"action": "navigate", "url": "https://example.com"}]"##.to_string();

        let actions = JsonActionSerializer.from(input).unwrap();
        let json = JsonActionSerializer.to(&actions).unwrap();
        let round_tripped = JsonActionSerializer.from(json).unwrap();

        assert_eq!(round_tripped.len(), actions.len());
    }
}
