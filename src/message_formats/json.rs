use crate::connection::Throughput as ThroughputEnum;
use crate::model::BenchmarkGroup;
use crate::report::{
    compare_to_threshold, BenchmarkId, ComparisonResult, MeasurementData, Report, ReportContext,
};
use crate::value_formatter::ValueFormatter;
use anyhow::Result;
use serde_derive::Serialize;
use serde_json::json;
use std::io::{stdout, Write};

use super::ConfidenceInterval;

trait Message: serde::ser::Serialize {
    fn reason() -> &'static str;
}

#[derive(Serialize)]
struct Throughput {
    per_iteration: u64,
    unit: String,
}
impl From<&ThroughputEnum> for Throughput {
    fn from(other: &ThroughputEnum) -> Self {
        match other {
            ThroughputEnum::Bytes(bytes) => Throughput {
                per_iteration: *bytes,
                unit: "bytes".to_owned(),
            },
            ThroughputEnum::Elements(elements) => Throughput {
                per_iteration: *elements,
                unit: "elements".to_owned(),
            },
        }
    }
}

#[derive(Serialize)]
enum ChangeType {
    NoChange,
    Improved,
    Regressed,
}

#[derive(Serialize)]
struct ChangeDetails {
    mean: ConfidenceInterval,
    median: ConfidenceInterval,

    change: ChangeType,
}

#[derive(Serialize)]
struct BenchmarkComplete {
    id: String,
    report_directory: String,
    iteration_count: Vec<u64>,
    measured_values: Vec<f64>,
    unit: String,

    throughput: Vec<Throughput>,

    typical: ConfidenceInterval,
    mean: ConfidenceInterval,
    median: ConfidenceInterval,
    median_abs_dev: ConfidenceInterval,
    slope: Option<ConfidenceInterval>,

    change: Option<ChangeDetails>,
}
impl Message for BenchmarkComplete {
    fn reason() -> &'static str {
        "benchmark-complete"
    }
}

#[derive(Serialize)]
struct BenchmarkGroupComplete {
    group_name: String,
    benchmarks: Vec<String>,
    report_directory: String,
}
impl Message for BenchmarkGroupComplete {
    fn reason() -> &'static str {
        "group-complete"
    }
}

pub struct JsonMessageReport;
impl JsonMessageReport {
    fn send_message<M: Message>(&self, message: M) {
        fn do_send<M: Message>(message: M) -> Result<()> {
            // Format the message to string
            let message_text = serde_json::to_string(&message)?;
            assert!(message_text.starts_with('{'));

            let reason = json!(M::reason());

            // Concatenate that into the message
            writeln!(stdout(), "{{\"reason\":{},{}", reason, &message_text[1..])?;
            Ok(())
        }
        if let Err(e) = do_send(message) {
            error!("Unexpected error writing JSON message: {:?}", e)
        }
    }
}
impl Report for JsonMessageReport {
    fn measurement_complete(
        &self,
        id: &BenchmarkId,
        context: &ReportContext,
        measurements: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        let mut measured_values = measurements.sample_times().to_vec();
        let unit = formatter.scale_for_machines(&mut measured_values);

        let iteration_count: Vec<u64> = measurements
            .iter_counts()
            .iter()
            .map(|count| *count as u64)
            .collect();

        let message = BenchmarkComplete {
            id: id.as_title().to_owned(),
            report_directory: path!(&context.output_directory, id.as_directory_name())
                .display()
                .to_string(),
            iteration_count,
            measured_values,
            unit,

            throughput: measurements
                .throughput
                .iter()
                .map(Throughput::from)
                .collect(),

            typical: ConfidenceInterval::from_estimate(
                measurements.absolute_estimates.typical(),
                formatter,
            ),
            mean: ConfidenceInterval::from_estimate(
                &measurements.absolute_estimates.mean,
                formatter,
            ),
            median: ConfidenceInterval::from_estimate(
                &measurements.absolute_estimates.median,
                formatter,
            ),
            median_abs_dev: ConfidenceInterval::from_estimate(
                &measurements.absolute_estimates.median_abs_dev,
                formatter,
            ),
            slope: measurements
                .absolute_estimates
                .slope
                .as_ref()
                .map(|slope| ConfidenceInterval::from_estimate(slope, formatter)),
            change: measurements.comparison.as_ref().map(|comparison| {
                let different_mean = comparison.p_value < comparison.significance_threshold;
                let mean_est = &comparison.relative_estimates.mean;

                let change = if !different_mean {
                    ChangeType::NoChange
                } else {
                    let comparison = compare_to_threshold(mean_est, comparison.noise_threshold);
                    match comparison {
                        ComparisonResult::Improved => ChangeType::Improved,
                        ComparisonResult::Regressed => ChangeType::Regressed,
                        ComparisonResult::NonSignificant => ChangeType::NoChange,
                    }
                };

                ChangeDetails {
                    mean: ConfidenceInterval::from_percent(&comparison.relative_estimates.mean),
                    median: ConfidenceInterval::from_percent(&comparison.relative_estimates.median),
                    change,
                }
            }),
        };

        self.send_message(message);
    }

    fn summarize(
        &self,
        context: &ReportContext,
        group_id: &str,
        benchmark_group: &BenchmarkGroup,
        _formatter: &ValueFormatter,
    ) {
        let message = BenchmarkGroupComplete {
            group_name: group_id.to_owned(),
            benchmarks: benchmark_group
                .benchmarks
                .keys()
                .map(|id| id.as_title().to_owned())
                .collect(),
            report_directory: path!(
                &context.output_directory,
                BenchmarkId::new(group_id.to_owned(), None, None, None).as_directory_name()
            )
            .display()
            .to_string(),
        };

        self.send_message(message);
    }
}
