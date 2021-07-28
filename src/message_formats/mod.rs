mod json;
use crate::config::{MessageFormat, SelfConfig};

use self::json::JsonMessageReport;

pub fn create_machine_report(self_config: &SelfConfig) -> Option<JsonMessageReport> {
    if let Some(MessageFormat::Json) = self_config.message_format {
        Some(JsonMessageReport)
    } else {
        None
    }
}
