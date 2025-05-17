use crate::connection::{PlotConfiguration, Throughput};
use crate::estimate::{ChangeDistributions, ChangeEstimates, Distributions, Estimate, Estimates};
use crate::format;
use crate::model::{BenchmarkGroup, Model, SavedStatistics};
use crate::stats::bivariate::regression::Slope;
use crate::stats::bivariate::Data;
use crate::stats::univariate::outliers::tukey::LabeledSample;
use crate::stats::univariate::Sample;
use crate::stats::Distribution;
use crate::value_formatter::ValueFormatter;
use std::cell::Cell;
use std::cmp;
use std::collections::HashSet;
use std::fmt;
use std::io::stderr;
use std::io::Write;
use std::path::{Path, PathBuf};

const MAX_DIRECTORY_NAME_LEN: usize = 64;
const MAX_TITLE_LEN: usize = 100;

pub struct ComparisonData {
    pub p_value: f64,
    pub t_distribution: Distribution<f64>,
    pub t_value: f64,
    pub relative_estimates: ChangeEstimates,
    pub relative_distributions: ChangeDistributions,
    pub significance_threshold: f64,
    pub noise_threshold: f64,
    pub base_iter_counts: Vec<f64>,
    pub base_sample_times: Vec<f64>,
    pub base_avg_times: Vec<f64>,
    pub base_estimates: Estimates,
}

pub struct MeasurementData<'a> {
    pub data: Data<'a, f64, f64>,
    pub avg_times: LabeledSample<'a, f64>,
    pub absolute_estimates: Estimates,
    pub distributions: Distributions,
    pub comparison: Option<ComparisonData>,
    pub throughput: Option<Throughput>,
}
impl MeasurementData<'_> {
    pub fn iter_counts(&self) -> &Sample<f64> {
        self.data.x()
    }

    pub fn sample_times(&self) -> &Sample<f64> {
        self.data.y()
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ValueType {
    Bytes,
    Elements,
    Value,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub struct BenchmarkId {
    pub group_id: String,
    pub function_id: Option<String>,
    pub value_str: Option<String>,
    pub throughput: Option<Throughput>,
    full_id: String,
    directory_name: PathBuf,
    title: String,
}

fn truncate_to_character_boundary(s: &mut String, max_len: usize) {
    let mut boundary = cmp::min(max_len, s.len());
    while !s.is_char_boundary(boundary) {
        boundary -= 1;
    }
    s.truncate(boundary);
}

pub fn make_filename_safe(string: &str) -> String {
    let mut string = string.replace(
        &['?', '"', '/', '\\', '*', '<', '>', ':', '|', '^'][..],
        "_",
    );

    // Truncate to last character boundary before max length...
    truncate_to_character_boundary(&mut string, MAX_DIRECTORY_NAME_LEN);

    if cfg!(target_os = "windows") {
        {
            string = string
                // On Windows, spaces in the end of the filename are ignored and will be trimmed.
                //
                // Without trimming ourselves, creating a directory `dir ` will silently create
                // `dir` instead, but then operations on files like `dir /file` will fail.
                //
                // Also note that it's important to do this *after* trimming to MAX_DIRECTORY_NAME_LEN,
                // otherwise it can trim again to a name with a trailing space.
                .trim_end()
                // On Windows, file names are not case-sensitive, so lowercase everything.
                .to_lowercase();
        }
    }

    string
}

impl BenchmarkId {
    pub fn new(
        group_id: String,
        function_id: Option<String>,
        value_str: Option<String>,
        throughput: Option<Throughput>,
    ) -> BenchmarkId {
        let full_id = match (&function_id, &value_str) {
            (Some(ref func), Some(ref val)) => format!("{}/{}/{}", group_id, func, val),
            (Some(ref func), &None) => format!("{}/{}", group_id, func),
            (&None, Some(ref val)) => format!("{}/{}", group_id, val),
            (&None, &None) => group_id.clone(),
        };

        let mut title = full_id.clone();
        truncate_to_character_boundary(&mut title, MAX_TITLE_LEN);
        if title != full_id {
            title.push_str("...");
        }

        let mut directory_name = PathBuf::from(make_filename_safe(&group_id));
        if let Some(func) = &function_id {
            directory_name.push(make_filename_safe(func));
        }
        if let Some(val) = &value_str {
            directory_name.push(make_filename_safe(val));
        }

        BenchmarkId {
            group_id,
            function_id,
            value_str,
            throughput,
            full_id,
            directory_name,
            title,
        }
    }

    pub fn as_title(&self) -> &str {
        &self.title
    }

    pub fn as_directory_name(&self) -> &Path {
        &self.directory_name
    }

    pub fn as_number(&self) -> Option<f64> {
        match self.throughput {
            Some(Throughput::Bytes(n))
            | Some(Throughput::BytesDecimal(n))
            | Some(Throughput::Elements(n)) => Some(n as f64),
            None => self
                .value_str
                .as_ref()
                .and_then(|string| string.parse::<f64>().ok()),
        }
    }

    pub fn value_type(&self) -> Option<ValueType> {
        match self.throughput {
            Some(Throughput::Bytes(_)) | Some(Throughput::BytesDecimal(_)) => {
                Some(ValueType::Bytes)
            }
            Some(Throughput::Elements(_)) => Some(ValueType::Elements),
            None => self
                .value_str
                .as_ref()
                .and_then(|string| string.parse::<f64>().ok())
                .map(|_| ValueType::Value),
        }
    }

    pub fn ensure_directory_name_unique(&mut self, existing_directories: &HashSet<PathBuf>) {
        if !existing_directories.contains(self.as_directory_name()) {
            return;
        }

        let mut counter = 2;
        loop {
            let mut file_name = self.as_directory_name().file_name().unwrap().to_os_string();
            file_name.push(format!("_{}", counter));
            let new_dir_name = self.as_directory_name().with_file_name(file_name);

            if !existing_directories.contains(&new_dir_name) {
                self.directory_name = new_dir_name;
                return;
            }
            counter += 1;
        }
    }

    pub fn ensure_title_unique(&mut self, existing_titles: &HashSet<String>) {
        if !existing_titles.contains(self.as_title()) {
            return;
        }

        let mut counter = 2;
        loop {
            let new_title = format!("{} #{}", self.as_title(), counter);
            if !existing_titles.contains(&new_title) {
                self.title = new_title;
                return;
            }
            counter += 1;
        }
    }
}
impl fmt::Display for BenchmarkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_title())
    }
}
impl fmt::Debug for BenchmarkId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fn format_opt(opt: &Option<String>) -> String {
            match *opt {
                Some(ref string) => format!("\"{}\"", string),
                None => "None".to_owned(),
            }
        }

        write!(
            f,
            "BenchmarkId {{ group_id: \"{}\", function_id: {}, value_str: {}, throughput: {:?} }}",
            self.group_id,
            format_opt(&self.function_id),
            format_opt(&self.value_str),
            self.throughput,
        )
    }
}

pub struct ReportContext {
    pub output_directory: PathBuf,
    pub plot_config: PlotConfiguration,
}
impl ReportContext {
    pub fn report_path<P: AsRef<Path> + ?Sized>(&self, id: &BenchmarkId, file_name: &P) -> PathBuf {
        path!(
            self.output_directory.clone(),
            id.as_directory_name(),
            file_name
        )
    }
}

pub trait Report {
    fn benchmark_start(&self, _id: &BenchmarkId, _context: &ReportContext) {}
    fn warmup(&self, _id: &BenchmarkId, _context: &ReportContext, _warmup_ns: f64) {}
    fn analysis(&self, _id: &BenchmarkId, _context: &ReportContext) {}
    fn measurement_start(
        &self,
        _id: &BenchmarkId,
        _context: &ReportContext,
        _sample_count: u64,
        _estimate_ns: f64,
        _iter_count: u64,
    ) {
    }
    fn measurement_complete(
        &self,
        _id: &BenchmarkId,
        _context: &ReportContext,
        _measurements: &MeasurementData<'_>,
        _formatter: &ValueFormatter,
    ) {
    }
    fn summarize(
        &self,
        _context: &ReportContext,
        _group_id: &str,
        _benchmark_group: &BenchmarkGroup,
        _formatter: &ValueFormatter,
    ) {
    }
    fn final_summary(&self, _context: &ReportContext, _model: &Model) {}
    fn group_separator(&self) {}
    fn history(
        &self,
        _context: &ReportContext,
        _id: &BenchmarkId,
        _history: &[SavedStatistics],
        _formatter: &ValueFormatter,
    ) {
    }
}

pub struct Reports<'a> {
    reports: Vec<&'a dyn Report>,
}
impl<'a> Reports<'a> {
    pub fn new(reports: Vec<&'a dyn Report>) -> Reports<'a> {
        Reports { reports }
    }
}
impl Report for Reports<'_> {
    fn benchmark_start(&self, id: &BenchmarkId, context: &ReportContext) {
        for report in &self.reports {
            report.benchmark_start(id, context);
        }
    }

    fn warmup(&self, id: &BenchmarkId, context: &ReportContext, warmup_ns: f64) {
        for report in &self.reports {
            report.warmup(id, context, warmup_ns);
        }
    }

    fn analysis(&self, id: &BenchmarkId, context: &ReportContext) {
        for report in &self.reports {
            report.analysis(id, context);
        }
    }

    fn measurement_start(
        &self,
        id: &BenchmarkId,
        context: &ReportContext,
        sample_count: u64,
        estimate_ns: f64,
        iter_count: u64,
    ) {
        for report in &self.reports {
            report.measurement_start(id, context, sample_count, estimate_ns, iter_count);
        }
    }

    fn measurement_complete(
        &self,
        id: &BenchmarkId,
        context: &ReportContext,
        measurements: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        for report in &self.reports {
            report.measurement_complete(id, context, measurements, formatter);
        }
    }

    fn summarize(
        &self,
        context: &ReportContext,
        group_id: &str,
        benchmark_group: &BenchmarkGroup,
        formatter: &ValueFormatter,
    ) {
        for report in &self.reports {
            report.summarize(context, group_id, benchmark_group, formatter);
        }
    }

    fn final_summary(&self, context: &ReportContext, model: &Model) {
        for report in &self.reports {
            report.final_summary(context, model);
        }
    }

    fn group_separator(&self) {
        for report in &self.reports {
            report.group_separator();
        }
    }

    fn history(
        &self,
        context: &ReportContext,
        id: &BenchmarkId,
        history: &[SavedStatistics],
        formatter: &ValueFormatter,
    ) {
        for report in &self.reports {
            report.history(context, id, history, formatter);
        }
    }
}

pub struct CliReport {
    pub enable_text_overwrite: bool,
    pub enable_text_coloring: bool,
    pub verbose: bool,
    pub show_differences: bool,

    last_line_len: Cell<usize>,
}
impl CliReport {
    pub fn new(
        enable_text_overwrite: bool,
        enable_text_coloring: bool,
        show_differences: bool,
        verbose: bool,
    ) -> CliReport {
        CliReport {
            enable_text_overwrite,
            enable_text_coloring,
            show_differences,
            verbose,

            last_line_len: Cell::new(0),
        }
    }

    fn text_overwrite(&self) {
        if self.enable_text_overwrite {
            eprint!("\r");
            for _ in 0..self.last_line_len.get() {
                eprint!(" ");
            }
            eprint!("\r");
        }
    }

    //Passing a String is the common case here.
    #[allow(clippy::needless_pass_by_value)]
    fn print_overwritable(&self, s: String) {
        if self.enable_text_overwrite {
            self.last_line_len.set(s.len());
            eprint!("{}", s);
            stderr().flush().unwrap();
        } else {
            eprintln!("{}", s);
        }
    }

    fn green(&self, s: String) -> String {
        if self.enable_text_coloring {
            format!("\x1B[32m{}\x1B[39m", s)
        } else {
            s
        }
    }

    fn yellow(&self, s: String) -> String {
        if self.enable_text_coloring {
            format!("\x1B[33m{}\x1B[39m", s)
        } else {
            s
        }
    }

    fn red(&self, s: String) -> String {
        if self.enable_text_coloring {
            format!("\x1B[31m{}\x1B[39m", s)
        } else {
            s
        }
    }

    fn bold(&self, s: String) -> String {
        if self.enable_text_coloring {
            format!("\x1B[1m{}\x1B[22m", s)
        } else {
            s
        }
    }

    fn faint(&self, s: String) -> String {
        if self.enable_text_coloring {
            format!("\x1B[2m{}\x1B[22m", s)
        } else {
            s
        }
    }

    pub fn outliers(&self, sample: &LabeledSample<'_, f64>) {
        let (los, lom, _, him, his) = sample.count();
        let noutliers = los + lom + him + his;
        let sample_size = sample.len();

        if noutliers == 0 {
            return;
        }

        let percent = |n: usize| 100. * n as f64 / sample_size as f64;

        eprintln!(
            "{}",
            self.yellow(format!(
                "Found {} outliers among {} measurements ({:.2}%)",
                noutliers,
                sample_size,
                percent(noutliers)
            ))
        );

        let print = |n, label| {
            if n != 0 {
                eprintln!("  {} ({:.2}%) {}", n, percent(n), label);
            }
        };

        print(los, "low severe");
        print(lom, "low mild");
        print(him, "high mild");
        print(his, "high severe");
    }
}
impl Report for CliReport {
    fn benchmark_start(&self, id: &BenchmarkId, _: &ReportContext) {
        self.print_overwritable(format!("Benchmarking {}", id));
    }

    fn warmup(&self, id: &BenchmarkId, _: &ReportContext, warmup_ns: f64) {
        self.text_overwrite();
        self.print_overwritable(format!(
            "Benchmarking {}: Warming up for {}",
            id,
            format::time(warmup_ns)
        ));
    }

    fn analysis(&self, id: &BenchmarkId, _: &ReportContext) {
        self.text_overwrite();
        self.print_overwritable(format!("Benchmarking {}: Analyzing", id));
    }

    fn measurement_start(
        &self,
        id: &BenchmarkId,
        _: &ReportContext,
        sample_count: u64,
        estimate_ns: f64,
        iter_count: u64,
    ) {
        self.text_overwrite();
        let iter_string = if self.verbose {
            format!("{} iterations", iter_count)
        } else {
            format::iter_count(iter_count)
        };

        self.print_overwritable(format!(
            "Benchmarking {}: Collecting {} samples in estimated {} ({})",
            id,
            sample_count,
            format::time(estimate_ns),
            iter_string
        ));
    }

    fn measurement_complete(
        &self,
        id: &BenchmarkId,
        _: &ReportContext,
        meas: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        self.text_overwrite();

        let typical_estimate = meas.absolute_estimates.typical();

        {
            let mut id = id.as_title().to_owned();

            if id.len() > 23 {
                eprintln!("{}", self.green(id.clone()));
                id.clear();
            }
            let id_len = id.len();

            eprintln!(
                "{}{}time:   [{} {} {}]",
                self.green(id),
                " ".repeat(24 - id_len),
                self.faint(
                    formatter.format_value(typical_estimate.confidence_interval.lower_bound)
                ),
                self.bold(formatter.format_value(typical_estimate.point_estimate)),
                self.faint(
                    formatter.format_value(typical_estimate.confidence_interval.upper_bound)
                )
            );
        }

        if let Some(ref throughput) = meas.throughput {
            eprintln!(
                "{}thrpt:  [{} {} {}]",
                " ".repeat(24),
                self.faint(formatter.format_throughput(
                    throughput,
                    typical_estimate.confidence_interval.upper_bound
                )),
                self.bold(formatter.format_throughput(throughput, typical_estimate.point_estimate)),
                self.faint(formatter.format_throughput(
                    throughput,
                    typical_estimate.confidence_interval.lower_bound
                )),
            )
        }

        if self.show_differences {
            if let Some(ref comp) = meas.comparison {
                let different_mean = comp.p_value < comp.significance_threshold;
                let mean_est = &comp.relative_estimates.mean;
                let point_estimate = mean_est.point_estimate;
                let mut point_estimate_str = format::change(point_estimate, true);
                // The change in throughput is related to the change in timing. Reducing the timing by
                // 50% increases the througput by 100%.
                let to_thrpt_estimate = |ratio: f64| 1.0 / (1.0 + ratio) - 1.0;
                let mut thrpt_point_estimate_str =
                    format::change(to_thrpt_estimate(point_estimate), true);
                let explanation_str: String;

                if !different_mean {
                    explanation_str = "No change in performance detected.".to_owned();
                } else {
                    let comparison = compare_to_threshold(mean_est, comp.noise_threshold);
                    match comparison {
                        ComparisonResult::Improved => {
                            point_estimate_str = self.green(self.bold(point_estimate_str));
                            thrpt_point_estimate_str =
                                self.green(self.bold(thrpt_point_estimate_str));
                            explanation_str =
                                format!("Performance has {}.", self.green("improved".to_owned()));
                        }
                        ComparisonResult::Regressed => {
                            point_estimate_str = self.red(self.bold(point_estimate_str));
                            thrpt_point_estimate_str =
                                self.red(self.bold(thrpt_point_estimate_str));
                            explanation_str =
                                format!("Performance has {}.", self.red("regressed".to_owned()));
                        }
                        ComparisonResult::NonSignificant => {
                            explanation_str = "Change within noise threshold.".to_owned();
                        }
                    }
                }

                if meas.throughput.is_some() {
                    eprintln!("{}change:", " ".repeat(17));

                    eprintln!(
                        "{}time:   [{} {} {}] (p = {:.2} {} {:.2})",
                        " ".repeat(24),
                        self.faint(format::change(
                            mean_est.confidence_interval.lower_bound,
                            true
                        )),
                        point_estimate_str,
                        self.faint(format::change(
                            mean_est.confidence_interval.upper_bound,
                            true
                        )),
                        comp.p_value,
                        if different_mean { "<" } else { ">" },
                        comp.significance_threshold
                    );
                    eprintln!(
                        "{}thrpt:  [{} {} {}]",
                        " ".repeat(24),
                        self.faint(format::change(
                            to_thrpt_estimate(mean_est.confidence_interval.upper_bound),
                            true
                        )),
                        thrpt_point_estimate_str,
                        self.faint(format::change(
                            to_thrpt_estimate(mean_est.confidence_interval.lower_bound),
                            true
                        )),
                    );
                } else {
                    eprintln!(
                        "{}change: [{} {} {}] (p = {:.2} {} {:.2})",
                        " ".repeat(24),
                        self.faint(format::change(
                            mean_est.confidence_interval.lower_bound,
                            true
                        )),
                        point_estimate_str,
                        self.faint(format::change(
                            mean_est.confidence_interval.upper_bound,
                            true
                        )),
                        comp.p_value,
                        if different_mean { "<" } else { ">" },
                        comp.significance_threshold
                    );
                }

                eprintln!("{}{}", " ".repeat(24), explanation_str);
            }
        }

        if self.verbose {
            self.outliers(&meas.avg_times);

            let format_short_estimate = |estimate: &Estimate| -> String {
                format!(
                    "[{} {}]",
                    formatter.format_value(estimate.confidence_interval.lower_bound),
                    formatter.format_value(estimate.confidence_interval.upper_bound)
                )
            };

            let data = &meas.data;
            if let Some(slope_estimate) = meas.absolute_estimates.slope.as_ref() {
                eprintln!(
                    "{:<7}{} {:<15}[{:0.7} {:0.7}]",
                    "slope",
                    format_short_estimate(slope_estimate),
                    "R^2",
                    Slope(slope_estimate.confidence_interval.lower_bound).r_squared(data),
                    Slope(slope_estimate.confidence_interval.upper_bound).r_squared(data),
                );
            }
            eprintln!(
                "{:<7}{} {:<15}{}",
                "mean",
                format_short_estimate(&meas.absolute_estimates.mean),
                "std. dev.",
                format_short_estimate(&meas.absolute_estimates.std_dev),
            );
            eprintln!(
                "{:<7}{} {:<15}{}",
                "median",
                format_short_estimate(&meas.absolute_estimates.median),
                "med. abs. dev.",
                format_short_estimate(&meas.absolute_estimates.median_abs_dev),
            );
        }
    }

    fn group_separator(&self) {
        eprintln!();
    }
}

pub struct BencherReport;
impl Report for BencherReport {
    fn measurement_start(
        &self,
        id: &BenchmarkId,
        _context: &ReportContext,
        _sample_count: u64,
        _estimate_ns: f64,
        _iter_count: u64,
    ) {
        eprint!("test {} ... ", id);
    }

    fn measurement_complete(
        &self,
        _id: &BenchmarkId,
        _: &ReportContext,
        meas: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        let mut values = [
            meas.absolute_estimates.median.point_estimate,
            meas.absolute_estimates.std_dev.point_estimate,
        ];
        let unit = formatter.scale_for_machines(&mut values);

        eprintln!(
            "bench: {:>11} {}/iter (+/- {})",
            format::integer(values[0]),
            unit,
            format::integer(values[1])
        );
    }

    fn group_separator(&self) {
        eprintln!();
    }
}

pub enum ComparisonResult {
    Improved,
    Regressed,
    NonSignificant,
}

pub fn compare_to_threshold(estimate: &Estimate, noise: f64) -> ComparisonResult {
    let ci = &estimate.confidence_interval;
    let lb = ci.lower_bound;
    let ub = ci.upper_bound;

    if lb < -noise && ub < -noise {
        ComparisonResult::Improved
    } else if lb > noise && ub > noise {
        ComparisonResult::Regressed
    } else {
        ComparisonResult::NonSignificant
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_make_filename_safe_replaces_characters() {
        let input = "?/\\*\"";
        let safe = make_filename_safe(input);
        assert_eq!("_____", &safe);
    }

    #[test]
    fn test_make_filename_safe_truncates_long_strings() {
        let input = "this is a very long string. it is too long to be safe as a directory name, and so it needs to be truncated. what a long string this is.";
        let safe = make_filename_safe(input);
        assert!(input.len() > MAX_DIRECTORY_NAME_LEN);
        assert_eq!(&input[0..MAX_DIRECTORY_NAME_LEN], &safe);
    }

    #[test]
    fn test_make_filename_safe_respects_character_boundaries() {
        let input = "✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓✓";
        let safe = make_filename_safe(input);
        assert!(safe.len() < MAX_DIRECTORY_NAME_LEN);
    }

    #[test]
    fn test_benchmark_id_make_directory_name_unique() {
        let existing_id = BenchmarkId::new(
            "group".to_owned(),
            Some("function".to_owned()),
            Some("value".to_owned()),
            None,
        );
        let mut directories = HashSet::new();
        directories.insert(existing_id.as_directory_name().to_owned());

        let mut new_id = existing_id.clone();
        new_id.ensure_directory_name_unique(&directories);
        assert_eq!(
            "group/function/value_2",
            new_id.as_directory_name().to_str().unwrap()
        );
        directories.insert(new_id.as_directory_name().to_owned());

        new_id = existing_id.clone();
        new_id.ensure_directory_name_unique(&directories);
        assert_eq!(
            "group/function/value_3",
            new_id.as_directory_name().to_str().unwrap()
        );
        directories.insert(new_id.as_directory_name().to_owned());
    }
    #[test]
    fn test_benchmark_id_make_long_directory_name_unique() {
        let long_name = (0..MAX_DIRECTORY_NAME_LEN).map(|_| 'a').collect::<String>();
        let existing_id = BenchmarkId::new(long_name, None, None, None);
        let mut directories = HashSet::new();
        directories.insert(existing_id.as_directory_name().to_owned());

        let mut new_id = existing_id.clone();
        new_id.ensure_directory_name_unique(&directories);
        assert_ne!(existing_id.as_directory_name(), new_id.as_directory_name());
    }
}
