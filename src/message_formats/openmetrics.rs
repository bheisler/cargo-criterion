use crate::report::{BenchmarkId, MeasurementData, Report, ReportContext};
use crate::value_formatter::ValueFormatter;

use super::ConfidenceInterval;

pub struct OpenMetricsMessageReport;

impl OpenMetricsMessageReport {
    fn print_confidence_interval(id: &BenchmarkId, metric: &ConfidenceInterval, name: &str) {
        let mut labels = vec![];

        if let Some(func) = &id.function_id {
            labels.push(("function", func.clone()));
        }

        if let Some(value) = &id.value_str {
            labels.push(("input_size", value.clone()));
        }

        labels.push(("aggregation", name.to_owned()));

        let labels = labels
            .into_iter()
            .map(|(key, value)| format!("{}=\"{}\"", key, value))
            .collect::<Vec<_>>()
            .join(",");

        println!(
            "criterion_benchmark_result_{}{{id=\"{}\",confidence=\"estimate\",{}}} {}",
            metric.unit, id.group_id, labels, metric.estimate
        );
        println!(
            "criterion_benchmark_result_{}{{id=\"{}\",confidence=\"upper_bound\",{}}} {}",
            metric.unit, id.group_id, labels, metric.upper_bound
        );
        println!(
            "criterion_benchmark_result_{}{{id=\"{}\",confidence=\"lower_bound\",{}}} {}",
            metric.unit, id.group_id, labels, metric.lower_bound
        );
    }
}

impl Report for OpenMetricsMessageReport {
    fn measurement_complete(
        &self,
        id: &BenchmarkId,
        context: &ReportContext,
        measurements: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        Self::print_confidence_interval(
            id,
            &ConfidenceInterval::from_estimate(
                measurements.absolute_estimates.typical(),
                formatter,
            ),
            "typical",
        );
        Self::print_confidence_interval(
            id,
            &ConfidenceInterval::from_estimate(&measurements.absolute_estimates.mean, formatter),
            "mean",
        );
        Self::print_confidence_interval(
            id,
            &ConfidenceInterval::from_estimate(&measurements.absolute_estimates.median, formatter),
            "median",
        );
        Self::print_confidence_interval(
            id,
            &ConfidenceInterval::from_estimate(
                &measurements.absolute_estimates.median_abs_dev,
                formatter,
            ),
            "median_abs_dev",
        );

        if let Some(slope) = measurements
            .absolute_estimates
            .slope
            .as_ref()
            .map(|slope| ConfidenceInterval::from_estimate(slope, formatter))
        {
            Self::print_confidence_interval(id, &slope, "slope");
        }

        let input_size = if let Some(input_size) = &id.value_str {
            format!("input_size=\"{}\",", input_size)
        } else {
            "".into()
        };

        let function = if let Some(function) = &id.function_id {
            format!("function=\"{}\",", function)
        } else {
            "".into()
        };

        println!(
            "criterion_benchmark_info{{id=\"{}\",{}{}report_directory=\"{}\"}} 1",
            id.group_id,
            input_size,
            function,
            path!(&context.output_directory, id.as_directory_name()).display()
        );
    }
}
