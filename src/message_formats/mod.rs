mod json;
use crate::config::{MessageFormat, SelfConfig};

use self::json::JsonMessageReport;

pub fn create_machine_report(self_config: &SelfConfig) -> Option<JsonMessageReport> {
    match self_config.message_format {
        Some(MessageFormat::Json) => Some(JsonMessageReport),
        None => None,
    }
}
