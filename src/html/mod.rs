use crate::analysis::BenchmarkConfig;
use crate::estimate::Estimate;
use crate::format;
use crate::model::{
    Benchmark as BenchmarkModel, BenchmarkGroup as GroupModel, Model, SavedStatistics,
};
use crate::plot::{PlotContext, Plotter};
use crate::report::{
    compare_to_threshold, make_filename_safe, BenchmarkId, ComparisonResult, MeasurementData,
    Report, ReportContext,
};
use crate::stats::bivariate::regression::Slope;
use crate::value_formatter::ValueFormatter;
use anyhow::{Context as AnyhowContext, Result};
use linked_hash_set::LinkedHashSet;
use serde::Serialize;
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::path::{Path, PathBuf};
use tinytemplate::TinyTemplate;

pub struct Size(pub usize, pub usize);
const THUMBNAIL_SIZE: Option<Size> = Some(Size(450, 300));

const COMMON_CSS: &'static str = include_str!("common.css");

fn save<D, P>(data: &D, path: &P) -> Result<()>
where
    D: Serialize + Debug,
    P: AsRef<Path> + Debug,
{
    let buf =
        serde_json::to_string(&data).with_context(|| format!("Unable to serialize {:?}", data))?;
    save_string(&buf, path)
}

fn save_string<P>(data: &str, path: &P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    use std::io::Write;

    File::create(path)
        .and_then(|mut f| f.write_all(data.as_bytes()))
        .with_context(|| format!("Unable to save file {:?}", path))?;

    Ok(())
}

fn mkdirp<P>(path: &P) -> Result<()>
where
    P: AsRef<Path> + Debug,
{
    std::fs::create_dir_all(path.as_ref())
        .with_context(|| format!("Unable to create directory {:?}", path))?;
    Ok(())
}

fn debug_context<S: Serialize + Debug>(path: &Path, context: &S) {
    if crate::debug_enabled() {
        let mut context_path = PathBuf::from(path);
        context_path.set_extension("json");
        info!("Writing report context to {:?}", context_path);
        let result = save(context, &context_path);
        if let Err(e) = result {
            error!("Failed to write report context debug output: {}", e);
        }
    }
}

#[derive(Serialize, Debug)]
struct Context {
    common_css: &'static str,

    title: String,
    confidence: String,

    thumbnail_width: usize,
    thumbnail_height: usize,

    slope: Option<ConfidenceInterval>,
    r2: ConfidenceInterval,
    mean: ConfidenceInterval,
    std_dev: ConfidenceInterval,
    median: ConfidenceInterval,
    mad: ConfidenceInterval,
    throughput: Option<ConfidenceInterval>,

    additional_plots: Vec<Plot>,

    comparison: Option<Comparison>,
}

#[derive(Serialize, Debug)]
struct IndividualBenchmark {
    name: String,
    path: PathBuf,
    regression_exists: bool,
}
impl IndividualBenchmark {
    fn from_id(
        output_directory: &Path,
        path_prefix: &str,
        id: &BenchmarkId,
    ) -> IndividualBenchmark {
        let regression_path = path!(output_directory, id.as_directory_name(), "regression.svg");

        IndividualBenchmark {
            name: id.as_title().to_owned(),
            path: path!(path_prefix, id.as_directory_name()),
            regression_exists: regression_path.exists(),
        }
    }
}

#[derive(Serialize, Debug)]
struct SummaryContext {
    common_css: &'static str,

    group_id: String,

    thumbnail_width: usize,
    thumbnail_height: usize,

    violin_plot: Option<String>,
    line_chart: Option<String>,

    benchmarks: Vec<IndividualBenchmark>,
}

#[derive(Serialize, Debug)]
struct ConfidenceInterval {
    lower: String,
    upper: String,
    point: String,
}

#[derive(Serialize, Debug)]
struct Plot {
    name: String,
    url: String,
}
impl Plot {
    fn new(name: &str, url: &str) -> Plot {
        Plot {
            name: name.to_owned(),
            url: url.to_owned(),
        }
    }
}

#[derive(Serialize, Debug)]
struct Comparison {
    p_value: String,
    inequality: String,
    significance_level: String,
    explanation: String,

    change: ConfidenceInterval,
    thrpt_change: Option<ConfidenceInterval>,
    additional_plots: Vec<Plot>,
}

fn if_exists(output_directory: &Path, path: &Path) -> Option<String> {
    let report_path = path.join("index.html");
    if PathBuf::from(output_directory).join(&report_path).is_file() {
        Some(report_path.to_string_lossy().to_string())
    } else {
        None
    }
}
#[derive(Serialize, Debug)]
struct ReportLink<'a> {
    name: &'a str,
    path: Option<String>,
}
impl<'a> ReportLink<'a> {
    // TODO: Would be nice if I didn't have to keep making these components filename-safe.
    fn group(output_directory: &Path, group_id: &'a str) -> ReportLink<'a> {
        let path = PathBuf::from(make_filename_safe(group_id));

        ReportLink {
            name: group_id,
            path: if_exists(output_directory, &path),
        }
    }

    fn function(output_directory: &Path, group_id: &str, function_id: &'a str) -> ReportLink<'a> {
        let mut path = PathBuf::from(make_filename_safe(group_id));
        path.push(make_filename_safe(function_id));

        ReportLink {
            name: function_id,
            path: if_exists(output_directory, &path),
        }
    }

    fn value(output_directory: &Path, group_id: &str, value_str: &'a str) -> ReportLink<'a> {
        let mut path = PathBuf::from(make_filename_safe(group_id));
        path.push(make_filename_safe(value_str));

        ReportLink {
            name: value_str,
            path: if_exists(output_directory, &path),
        }
    }

    fn individual(output_directory: &Path, id: &'a BenchmarkId) -> ReportLink<'a> {
        let path = PathBuf::from(id.as_directory_name());
        ReportLink {
            name: id.as_title(),
            path: if_exists(output_directory, &path),
        }
    }
}

#[derive(Serialize, Debug)]
struct BenchmarkValueGroup<'a> {
    value: Option<ReportLink<'a>>,
    benchmarks: Vec<ReportLink<'a>>,
}

#[derive(Serialize, Debug)]
struct BenchmarkGroup<'a> {
    group_report: ReportLink<'a>,

    function_ids: Option<Vec<ReportLink<'a>>>,
    values: Option<Vec<ReportLink<'a>>>,

    individual_links: Vec<BenchmarkValueGroup<'a>>,
}
impl<'a> BenchmarkGroup<'a> {
    fn new(
        output_directory: &Path,
        group_id: &'a str,
        group: &'a GroupModel,
    ) -> BenchmarkGroup<'a> {
        let group_report = ReportLink::group(output_directory, group_id);

        let mut function_ids = LinkedHashSet::new();
        let mut values = LinkedHashSet::new();
        let mut individual_links = HashMap::with_capacity(group.benchmarks.len());

        for id in group.benchmarks.keys() {
            let function_id = id.function_id.as_deref();
            let value = id.value_str.as_deref();

            let individual_link = ReportLink::individual(output_directory, id);

            function_ids.insert_if_absent(function_id);
            values.insert_if_absent(value);

            individual_links.insert((function_id, value), individual_link);
        }

        let mut value_groups = Vec::with_capacity(values.len());
        for value in values.iter() {
            let row = function_ids
                .iter()
                .filter_map(|f| individual_links.remove(&(*f, *value)))
                .collect::<Vec<_>>();
            value_groups.push(BenchmarkValueGroup {
                value: value.map(|s| ReportLink::value(output_directory, group_id, s)),
                benchmarks: row,
            });
        }

        let function_ids = function_ids
            .into_iter()
            .map(|os| os.map(|s| ReportLink::function(output_directory, group_id, s)))
            .collect::<Option<Vec<_>>>();
        let values = values
            .into_iter()
            .map(|os| os.map(|s| ReportLink::value(output_directory, group_id, s)))
            .collect::<Option<Vec<_>>>();

        BenchmarkGroup {
            group_report,
            function_ids,
            values,
            individual_links: value_groups,
        }
    }
}

#[derive(Serialize, Debug)]
struct IndexContext<'a> {
    common_css: &'static str,
    groups: Vec<BenchmarkGroup<'a>>,
}

pub struct Html {
    templates: TinyTemplate<'static>,
    plotter: RefCell<Box<dyn Plotter>>,
}
impl Html {
    pub(crate) fn new(plotter: Box<dyn Plotter>) -> Html {
        let mut templates = TinyTemplate::new();
        templates
            .add_template("report_link", include_str!("report_link.html.tt"))
            .expect("Unable to parse report_link template.");
        templates
            .add_template("index", include_str!("index.html.tt"))
            .expect("Unable to parse index template.");
        templates
            .add_template("benchmark_report", include_str!("benchmark_report.html.tt"))
            .expect("Unable to parse benchmark_report template");
        templates
            .add_template("summary_report", include_str!("summary_report.html.tt"))
            .expect("Unable to parse summary_report template");

        let plotter = RefCell::new(plotter);
        Html { templates, plotter }
    }
}
impl Report for Html {
    fn measurement_complete(
        &self,
        id: &BenchmarkId,
        report_context: &ReportContext,
        measurements: &MeasurementData<'_>,
        formatter: &ValueFormatter,
    ) {
        try_else_return!({
            let report_dir = path!(&report_context.output_directory, id.as_directory_name());
            mkdirp(&report_dir)
        });

        let typical_estimate = measurements.absolute_estimates.typical();

        let time_interval = |est: &Estimate| -> ConfidenceInterval {
            ConfidenceInterval {
                lower: formatter.format_value(est.confidence_interval.lower_bound),
                point: formatter.format_value(est.point_estimate),
                upper: formatter.format_value(est.confidence_interval.upper_bound),
            }
        };

        let data = measurements.data;

        elapsed! {
            "Generating plots",
            self.generate_plots(id, report_context, formatter, measurements)
        }

        let throughput = measurements
            .throughput
            .as_ref()
            .map(|thr| ConfidenceInterval {
                lower: formatter
                    .format_throughput(thr, typical_estimate.confidence_interval.upper_bound),
                upper: formatter
                    .format_throughput(thr, typical_estimate.confidence_interval.lower_bound),
                point: formatter.format_throughput(thr, typical_estimate.point_estimate),
            });

        let mut additional_plots = vec![
            Plot::new("Typical", "typical.svg"),
            Plot::new("Mean", "mean.svg"),
            Plot::new("Std. Dev.", "SD.svg"),
            Plot::new("Median", "median.svg"),
            Plot::new("MAD", "MAD.svg"),
        ];
        if measurements.absolute_estimates.slope.is_some() {
            additional_plots.push(Plot::new("Slope", "slope.svg"));
        }

        let context = Context {
            common_css: COMMON_CSS,

            title: id.as_title().to_owned(),
            confidence: format!(
                "{:.2}",
                typical_estimate.confidence_interval.confidence_level
            ),

            thumbnail_width: THUMBNAIL_SIZE.unwrap().0,
            thumbnail_height: THUMBNAIL_SIZE.unwrap().1,

            slope: measurements
                .absolute_estimates
                .slope
                .as_ref()
                .map(time_interval),
            mean: time_interval(&measurements.absolute_estimates.mean),
            median: time_interval(&measurements.absolute_estimates.median),
            mad: time_interval(&measurements.absolute_estimates.median_abs_dev),
            std_dev: time_interval(&measurements.absolute_estimates.std_dev),
            throughput,

            r2: ConfidenceInterval {
                lower: format!(
                    "{:0.7}",
                    Slope(typical_estimate.confidence_interval.lower_bound).r_squared(&data)
                ),
                upper: format!(
                    "{:0.7}",
                    Slope(typical_estimate.confidence_interval.upper_bound).r_squared(&data)
                ),
                point: format!(
                    "{:0.7}",
                    Slope(typical_estimate.point_estimate).r_squared(&data)
                ),
            },

            additional_plots,

            comparison: self.comparison(measurements),
        };

        let report_path = path!(
            &report_context.output_directory,
            id.as_directory_name(),
            "index.html"
        );
        debug_context(&report_path, &context);

        let text = self
            .templates
            .render("benchmark_report", &context)
            .expect("Failed to render benchmark report template");
        try_else_return!(save_string(&text, &report_path));
    }

    fn summarize(
        &self,
        context: &ReportContext,
        group_id: &str,
        benchmark_group: &GroupModel,
        formatter: &ValueFormatter,
    ) {
        if benchmark_group.benchmarks.is_empty() {
            return;
        }

        let mut function_ids = LinkedHashSet::new();
        let mut value_strs = LinkedHashSet::new();
        for id in benchmark_group.benchmarks.keys() {
            if let Some(ref function_id) = id.function_id {
                function_ids.insert_if_absent(function_id);
            }
            if let Some(ref value_str) = id.value_str {
                value_strs.insert_if_absent(value_str);
            }
        }

        for function_id in function_ids {
            let samples_with_function: Vec<_> = benchmark_group
                .benchmarks
                .iter()
                .filter(|(ref id, _)| id.function_id.as_ref() == Some(&function_id))
                .collect();

            if samples_with_function.len() > 1 {
                let subgroup_id =
                    BenchmarkId::new(group_id.to_owned(), Some(function_id.clone()), None, None);

                self.generate_summary(
                    &subgroup_id,
                    &*samples_with_function,
                    context,
                    formatter,
                    false,
                );
            }
        }

        for value_str in value_strs {
            let samples_with_value: Vec<_> = benchmark_group
                .benchmarks
                .iter()
                .by_ref()
                .filter(|(ref id, _)| id.value_str.as_ref() == Some(&value_str))
                .collect();

            if samples_with_value.len() > 1 {
                let subgroup_id =
                    BenchmarkId::new(group_id.to_owned(), None, Some(value_str.clone()), None);

                self.generate_summary(
                    &subgroup_id,
                    &*samples_with_value,
                    context,
                    formatter,
                    false,
                );
            }
        }

        let all_data: Vec<_> = benchmark_group.benchmarks.iter().collect();

        self.generate_summary(
            &BenchmarkId::new(group_id.to_owned(), None, None, None),
            &*(all_data),
            context,
            formatter,
            true,
        );
        self.plotter.borrow_mut().wait();
    }

    fn final_summary(&self, report_context: &ReportContext, model: &Model) {
        let output_directory = &report_context.output_directory;

        let groups = model
            .groups
            .iter()
            .map(|(id, group)| BenchmarkGroup::new(output_directory, id, &group))
            .collect::<Vec<BenchmarkGroup<'_>>>();

        try_else_return!(mkdirp(&output_directory));

        let report_path = output_directory.join("index.html");

        let context = IndexContext {
            common_css: COMMON_CSS,
            groups,
        };

        debug_context(&report_path, &context);

        let text = self
            .templates
            .render("index", &context)
            .expect("Failed to render index template");
        try_else_return!(save_string(&text, &report_path,));
    }

    fn history(
        &self,
        context: &ReportContext,
        id: &BenchmarkId,
        history: &[SavedStatistics],
        config: &BenchmarkConfig,
        formatter: &ValueFormatter,
    ) {
    }
}
impl Html {
    fn comparison(&self, measurements: &MeasurementData<'_>) -> Option<Comparison> {
        if let Some(ref comp) = measurements.comparison {
            let different_mean = comp.p_value < comp.significance_threshold;
            let mean_est = &comp.relative_estimates.mean;
            let explanation_str: String;

            if !different_mean {
                explanation_str = "No change in performance detected.".to_owned();
            } else {
                let comparison = compare_to_threshold(&mean_est, comp.noise_threshold);
                match comparison {
                    ComparisonResult::Improved => {
                        explanation_str = "Performance has improved.".to_owned();
                    }
                    ComparisonResult::Regressed => {
                        explanation_str = "Performance has regressed.".to_owned();
                    }
                    ComparisonResult::NonSignificant => {
                        explanation_str = "Change within noise threshold.".to_owned();
                    }
                }
            }

            let comp = Comparison {
                p_value: format!("{:.2}", comp.p_value),
                inequality: (if different_mean { "<" } else { ">" }).to_owned(),
                significance_level: format!("{:.2}", comp.significance_threshold),
                explanation: explanation_str,

                change: ConfidenceInterval {
                    point: format::change(mean_est.point_estimate, true),
                    lower: format::change(mean_est.confidence_interval.lower_bound, true),
                    upper: format::change(mean_est.confidence_interval.upper_bound, true),
                },

                thrpt_change: measurements.throughput.as_ref().map(|_| {
                    let to_thrpt_estimate = |ratio: f64| 1.0 / (1.0 + ratio) - 1.0;
                    ConfidenceInterval {
                        point: format::change(to_thrpt_estimate(mean_est.point_estimate), true),
                        lower: format::change(
                            to_thrpt_estimate(mean_est.confidence_interval.lower_bound),
                            true,
                        ),
                        upper: format::change(
                            to_thrpt_estimate(mean_est.confidence_interval.upper_bound),
                            true,
                        ),
                    }
                }),

                additional_plots: vec![
                    Plot::new("Change in mean", "change/mean.svg"),
                    Plot::new("Change in median", "change/median.svg"),
                    Plot::new("T-Test", "change/t-test.svg"),
                ],
            };
            Some(comp)
        } else {
            None
        }
    }

    fn generate_plots(
        &self,
        id: &BenchmarkId,
        context: &ReportContext,
        formatter: &ValueFormatter,
        measurements: &MeasurementData,
    ) {
        let plot_ctx = PlotContext {
            id,
            context,
            size: None,
            is_thumbnail: false,
        };

        let plot_ctx_small = plot_ctx.thumbnail(true).size(THUMBNAIL_SIZE);

        self.plotter
            .borrow_mut()
            .pdf(plot_ctx, measurements, formatter);
        self.plotter
            .borrow_mut()
            .pdf_thumbnail(plot_ctx_small, measurements, formatter);
        if measurements.absolute_estimates.slope.is_some() {
            self.plotter
                .borrow_mut()
                .regression(plot_ctx, measurements, formatter);
            self.plotter
                .borrow_mut()
                .regression_thumbnail(plot_ctx_small, measurements, formatter);
        } else {
            self.plotter
                .borrow_mut()
                .iteration_times(plot_ctx, measurements, formatter);
            self.plotter.borrow_mut().iteration_times_thumbnail(
                plot_ctx_small,
                measurements,
                formatter,
            );
        }

        self.plotter
            .borrow_mut()
            .abs_distributions(plot_ctx, measurements, formatter);

        if let Some(ref comparison) = measurements.comparison {
            try_else_return!({
                let change_dir = path!(&context.output_directory, id.as_directory_name(), "change");
                mkdirp(&change_dir)
            });

            try_else_return!({
                let both_dir = path!(&context.output_directory, id.as_directory_name(), "both");
                mkdirp(&both_dir)
            });

            self.plotter
                .borrow_mut()
                .pdf_comparison(plot_ctx, measurements, formatter, comparison);
            self.plotter.borrow_mut().pdf_comparison_thumbnail(
                plot_ctx_small,
                measurements,
                formatter,
                comparison,
            );
            if measurements.absolute_estimates.slope.is_some()
                && comparison.base_estimates.slope.is_some()
            {
                self.plotter.borrow_mut().regression_comparison(
                    plot_ctx,
                    measurements,
                    formatter,
                    comparison,
                );
                self.plotter.borrow_mut().regression_comparison_thumbnail(
                    plot_ctx_small,
                    measurements,
                    formatter,
                    comparison,
                );
            } else {
                self.plotter.borrow_mut().iteration_times_comparison(
                    plot_ctx,
                    measurements,
                    formatter,
                    comparison,
                );
                self.plotter
                    .borrow_mut()
                    .iteration_times_comparison_thumbnail(
                        plot_ctx_small,
                        measurements,
                        formatter,
                        comparison,
                    );
            }
            self.plotter.borrow_mut().t_test(plot_ctx, comparison);
            self.plotter
                .borrow_mut()
                .rel_distributions(plot_ctx, comparison);
        }

        self.plotter.borrow_mut().wait();
    }

    fn generate_summary(
        &self,
        id: &BenchmarkId,
        data: &[(&BenchmarkId, &BenchmarkModel)],
        report_context: &ReportContext,
        formatter: &ValueFormatter,
        full_summary: bool,
    ) {
        let plot_ctx = PlotContext {
            id,
            context: report_context,
            size: None,
            is_thumbnail: false,
        };

        try_else_return!(
            {
                let report_dir = path!(&report_context.output_directory, id.as_directory_name());
                mkdirp(&report_dir)
            },
            || {}
        );

        self.plotter.borrow_mut().violin(plot_ctx, formatter, data);

        let value_types: Vec<_> = data.iter().map(|(ref id, _)| id.value_type()).collect();
        let mut line_path = None;

        if value_types.iter().all(|x| x == &value_types[0]) {
            if let Some(value_type) = value_types[0] {
                let values: Vec<_> = data.iter().map(|(ref id, _)| id.as_number()).collect();
                if values.iter().any(|x| x != &values[0]) {
                    self.plotter
                        .borrow_mut()
                        .line_comparison(plot_ctx, formatter, data, value_type);
                    line_path = Some(plot_ctx.line_comparison_path());
                }
            }
        }

        let path_prefix = if full_summary { ".." } else { "../.." };
        let benchmarks = data
            .iter()
            .map(|(ref id, _)| {
                IndividualBenchmark::from_id(&report_context.output_directory, path_prefix, id)
            })
            .collect();

        let context = SummaryContext {
            common_css: COMMON_CSS,
            group_id: id.as_title().to_owned(),

            thumbnail_width: THUMBNAIL_SIZE.unwrap().0,
            thumbnail_height: THUMBNAIL_SIZE.unwrap().1,

            violin_plot: Some(plot_ctx.violin_path().to_string_lossy().into_owned()),
            line_chart: line_path.map(|p| p.to_string_lossy().into_owned()),

            benchmarks,
        };

        let report_path = path!(
            &report_context.output_directory,
            id.as_directory_name(),
            "index.html"
        );
        debug_context(&report_path, &context);

        let text = self
            .templates
            .render("summary_report", &context)
            .expect("Failed to render summary report template");
        try_else_return!(save_string(&text, &report_path,), || {});
    }
}
