mod json;
mod openmetrics;

use crate::config::{MessageFormat, SelfConfig};
use crate::estimate::Estimate;
use crate::report::Report;
use crate::value_formatter::ValueFormatter;

use self::json::JsonMessageReport;
use self::openmetrics::OpenMetricsMessageReport;

#[derive(Serialize)]
struct ConfidenceInterval {
    estimate: f64,
    lower_bound: f64,
    upper_bound: f64,
    unit: String,
}
impl ConfidenceInterval {
    fn from_estimate(estimate: &Estimate, value_formatter: &ValueFormatter) -> ConfidenceInterval {
        let mut array = [
            estimate.point_estimate,
            estimate.confidence_interval.lower_bound,
            estimate.confidence_interval.upper_bound,
        ];
        let unit = value_formatter.scale_for_machines(&mut array);
        let [estimate, lower_bound, upper_bound] = array;
        ConfidenceInterval {
            estimate,
            lower_bound,
            upper_bound,
            unit,
        }
    }
    fn from_percent(estimate: &Estimate) -> ConfidenceInterval {
        ConfidenceInterval {
            estimate: estimate.point_estimate,
            lower_bound: estimate.confidence_interval.lower_bound,
            upper_bound: estimate.confidence_interval.upper_bound,
            unit: "%".to_owned(),
        }
    }
}

pub enum MessageReport {
    Json(JsonMessageReport),
    OpenMetrics(OpenMetricsMessageReport),
}
impl Report for MessageReport {
    fn measurement_complete(
        &self,
        id: &crate::report::BenchmarkId,
        context: &crate::report::ReportContext,
        measurements: &crate::report::MeasurementData<'_>,
        formatter: &crate::value_formatter::ValueFormatter,
    ) {
        match self {
            Self::Json(report) => report.measurement_complete(id, context, measurements, formatter),
            Self::OpenMetrics(report) => {
                report.measurement_complete(id, context, measurements, formatter)
            }
        }
    }

    fn summarize(
        &self,
        context: &crate::report::ReportContext,
        group_id: &str,
        benchmark_group: &crate::model::BenchmarkGroup,
        formatter: &crate::value_formatter::ValueFormatter,
    ) {
        match self {
            Self::Json(report) => report.summarize(context, group_id, benchmark_group, formatter),
            Self::OpenMetrics(report) => {
                report.summarize(context, group_id, benchmark_group, formatter)
            }
        }
    }
}

pub fn create_machine_report(self_config: &SelfConfig) -> Option<MessageReport> {
    match self_config.message_format {
        Some(MessageFormat::Json) => Some(MessageReport::Json(JsonMessageReport)),
        Some(MessageFormat::OpenMetrics) => {
            Some(MessageReport::OpenMetrics(OpenMetricsMessageReport))
        }
        None => None,
    }
}
