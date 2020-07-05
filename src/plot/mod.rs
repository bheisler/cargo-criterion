#[cfg(feature = "gnuplot_backend")]
mod gnuplot_backend;
#[cfg(feature = "plotters_backend")]
mod plotters_backend;

#[cfg(feature = "gnuplot_backend")]
pub use gnuplot_backend::Gnuplot;
#[cfg(feature = "plotters_backend")]
pub use plotters_backend::PlottersBackend;

use crate::estimate::Estimate;
use crate::estimate::Statistic;
use crate::kde;
use crate::model::Benchmark;
use crate::report::{BenchmarkId, ComparisonData, MeasurementData, ReportContext, ValueType};
use crate::stats::univariate::Sample;
use crate::stats::Distribution;
use crate::value_formatter::ValueFormatter;
use std::path::PathBuf;

const REPORT_STATS: [Statistic; 7] = [
    Statistic::Typical,
    Statistic::Slope,
    Statistic::Mean,
    Statistic::Median,
    Statistic::MedianAbsDev,
    Statistic::MedianAbsDev,
    Statistic::StdDev,
];
const CHANGE_STATS: [Statistic; 2] = [Statistic::Mean, Statistic::Median];

#[derive(Clone, Copy)]
pub struct PlotContext<'a> {
    pub id: &'a BenchmarkId,
    pub context: &'a ReportContext,
    pub size: Option<Size>,
    pub is_thumbnail: bool,
}

const KDE_POINTS: usize = 500;

#[derive(Debug, Clone, Copy)]
pub struct Size(pub usize, pub usize);

impl<'a> PlotContext<'a> {
    pub fn size(mut self, s: Option<crate::html::Size>) -> PlotContext<'a> {
        if let Some(s) = s {
            self.size = Some(Size(s.0, s.1));
        }
        self
    }

    pub fn thumbnail(mut self, value: bool) -> PlotContext<'a> {
        self.is_thumbnail = value;
        self
    }

    pub fn line_comparison_path(&self) -> PathBuf {
        path!(
            &self.context.output_directory,
            self.id.as_directory_name(),
            "lines.svg"
        )
    }

    pub fn violin_path(&self) -> PathBuf {
        path!(
            &self.context.output_directory,
            self.id.as_directory_name(),
            "violin.svg"
        )
    }
}

#[derive(Clone, Copy)]
pub struct PlotData<'a> {
    pub formatter: &'a dyn ValueFormatter,
    pub measurements: &'a MeasurementData<'a>,
    pub comparison: Option<&'a ComparisonData>,
}

impl<'a> PlotData<'a> {
    pub fn comparison(mut self, comp: &'a ComparisonData) -> PlotData<'a> {
        self.comparison = Some(comp);
        self
    }
}

pub trait Plotter {
    fn pdf(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn pdf_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn pdf_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn pdf_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn iteration_times(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn iteration_times_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn iteration_times_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn iteration_times_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn regression_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn regression_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);
    fn regression_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn abs_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn rel_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    );

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
    );

    fn t_test(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn wait(&mut self);
}

// Some types representing things we might want to draw

pub struct Point {
    x: f64,
    y: f64,
}

pub struct Line {
    pub start: Point,
    pub end: Point,
}

pub struct LineCurve<'a> {
    xs: &'a [f64],
    ys: &'a [f64],
}

pub struct FilledCurve<'a> {
    xs: &'a [f64],
    ys_1: &'a [f64],
    ys_2: &'a [f64],
}

pub trait PlottingBackend {
    fn abs_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        x_unit: &str,
        distribution_curve: LineCurve,
        bootstrap_area: FilledCurve,
        point_estimate: Line,
    );

    fn rel_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        distribution_curve: LineCurve,
        confidence_interval: FilledCurve,
        point_estimate: Line,
        noise_threshold: FilledCurve,
    );

    fn wait(&mut self);
}

pub struct PlotGenerator<B: PlottingBackend> {
    pub backend: B,
    pub fallback: Box<dyn Plotter>,
}
impl<B: PlottingBackend> PlotGenerator<B> {
    fn abs_distribution(
        &mut self,
        id: &BenchmarkId,
        context: &ReportContext,
        formatter: &dyn ValueFormatter,
        statistic: Statistic,
        distribution: &Distribution<f64>,
        estimate: &Estimate,
        size: Option<Size>,
    ) {
        let ci = &estimate.confidence_interval;
        let typical = ci.upper_bound;
        let mut ci_values = [ci.lower_bound, ci.upper_bound, estimate.point_estimate];
        let unit = formatter.scale_values(typical, &mut ci_values);
        let (lb, ub, point) = (ci_values[0], ci_values[1], ci_values[2]);

        let start = lb - (ub - lb) / 9.;
        let end = ub + (ub - lb) / 9.;
        let mut scaled_xs: Vec<f64> = distribution.iter().cloned().collect();
        let _ = formatter.scale_values(typical, &mut scaled_xs);
        let scaled_xs_sample = Sample::new(&scaled_xs);
        let (kde_xs, ys) = kde::sweep(scaled_xs_sample, KDE_POINTS, Some((start, end)));

        // interpolate between two points of the KDE sweep to find the Y position at the point estimate.
        let n_point = kde_xs
            .iter()
            .position(|&x| x >= point)
            .unwrap_or(kde_xs.len() - 1)
            .max(1); // Must be at least the second element or this will panic
        let slope = (ys[n_point] - ys[n_point - 1]) / (kde_xs[n_point] - kde_xs[n_point - 1]);
        let y_point = ys[n_point - 1] + (slope * (point - kde_xs[n_point - 1]));

        let start = kde_xs
            .iter()
            .enumerate()
            .find(|&(_, &x)| x >= lb)
            .unwrap()
            .0;
        let end = kde_xs
            .iter()
            .enumerate()
            .rev()
            .find(|&(_, &x)| x <= ub)
            .unwrap()
            .0;
        let len = end - start;

        let distribution_curve = LineCurve {
            xs: &*kde_xs,
            ys: &*ys,
        };
        let bootstrap_area = FilledCurve {
            xs: &kde_xs[start..end],
            ys_1: &ys[start..end],
            ys_2: &vec![0.0; len],
        };
        let estimate = Line {
            start: Point { x: point, y: 0.0 },
            end: Point {
                x: point,
                y: y_point,
            },
        };

        self.backend.abs_distribution(
            id,
            statistic,
            size,
            context.report_path(id, &format!("{}.svg", statistic)),
            &unit,
            distribution_curve,
            bootstrap_area,
            estimate,
        );
    }

    fn rel_distribution(
        &mut self,
        id: &BenchmarkId,
        context: &ReportContext,
        statistic: Statistic,
        distribution: &Distribution<f64>,
        estimate: &Estimate,
        noise_threshold: f64,
        size: Option<Size>,
    ) {
        let ci = &estimate.confidence_interval;
        let (lb, ub) = (ci.lower_bound, ci.upper_bound);

        let start = lb - (ub - lb) / 9.;
        let end = ub + (ub - lb) / 9.;
        let (xs, ys) = kde::sweep(distribution, KDE_POINTS, Some((start, end)));
        let xs_ = Sample::new(&xs);

        // interpolate between two points of the KDE sweep to find the Y position at the point estimate.
        let point = estimate.point_estimate;
        let n_point = xs
            .iter()
            .position(|&x| x >= point)
            .unwrap_or(ys.len() - 1)
            .max(1);
        let slope = (ys[n_point] - ys[n_point - 1]) / (xs[n_point] - xs[n_point - 1]);
        let y_point = ys[n_point - 1] + (slope * (point - xs[n_point - 1]));

        let start = xs.iter().enumerate().find(|&(_, &x)| x >= lb).unwrap().0;
        let end = xs
            .iter()
            .enumerate()
            .rev()
            .find(|&(_, &x)| x <= ub)
            .unwrap()
            .0;
        let len = end - start;

        let x_min = xs_.min();
        let x_max = xs_.max();

        let (fc_start, fc_end) = if noise_threshold < x_min || -noise_threshold > x_max {
            let middle = (x_min + x_max) / 2.;

            (middle, middle)
        } else {
            (
                if -noise_threshold < x_min {
                    x_min
                } else {
                    -noise_threshold
                },
                if noise_threshold > x_max {
                    x_max
                } else {
                    noise_threshold
                },
            )
        };

        let distribution_curve = LineCurve { xs: &*xs, ys: &*ys };
        let confidence_interval = FilledCurve {
            xs: &xs[start..end],
            ys_1: &ys[start..end],
            ys_2: &vec![0.0; len],
        };
        let estimate = Line {
            start: Point { x: point, y: 0.0 },
            end: Point {
                x: point,
                y: y_point,
            },
        };
        let noise_threshold = FilledCurve {
            xs: &[fc_start, fc_end],
            ys_1: &[1.0, 1.0],
            ys_2: &[0.0, 0.0],
        };

        self.backend.rel_distribution(
            id,
            statistic,
            size,
            context.report_path(id, &format!("change/{}.svg", statistic)),
            distribution_curve,
            confidence_interval,
            estimate,
            noise_threshold,
        );
    }
}
impl<B: PlottingBackend> Plotter for PlotGenerator<B> {
    fn pdf(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.pdf(ctx, data);
    }
    fn pdf_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.pdf_thumbnail(ctx, data);
    }
    fn pdf_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.pdf_comparison(ctx, data);
    }
    fn pdf_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.pdf_comparison_thumbnail(ctx, data);
    }

    fn iteration_times(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.iteration_times(ctx, data);
    }
    fn iteration_times_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.iteration_times_thumbnail(ctx, data);
    }
    fn iteration_times_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.iteration_times_comparison(ctx, data);
    }
    fn iteration_times_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback
            .iteration_times_comparison_thumbnail(ctx, data);
    }

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.regression(ctx, data);
    }
    fn regression_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.regression_thumbnail(ctx, data)
    }
    fn regression_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.regression_comparison(ctx, data)
    }
    fn regression_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.regression_comparison_thumbnail(ctx, data)
    }

    fn abs_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        REPORT_STATS
            .iter()
            .filter_map(|stat| {
                data.measurements.distributions.get(*stat).and_then(|dist| {
                    data.measurements
                        .absolute_estimates
                        .get(*stat)
                        .map(|est| (*stat, dist, est))
                })
            })
            .for_each(|(statistic, distribution, estimate)| {
                self.abs_distribution(
                    ctx.id,
                    ctx.context,
                    data.formatter,
                    statistic,
                    distribution,
                    estimate,
                    ctx.size,
                )
            })
    }

    fn rel_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let comparison = data
            .comparison
            .expect("Should not call rel_distributions without comparison data.");
        crate::plot::CHANGE_STATS.iter().for_each(|&statistic| {
            self.rel_distribution(
                ctx.id,
                ctx.context,
                statistic,
                comparison.relative_distributions.get(statistic),
                comparison.relative_estimates.get(statistic),
                comparison.noise_threshold,
                ctx.size,
            )
        });
    }

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    ) {
        self.fallback
            .line_comparison(ctx, formatter, all_curves, value_type);
    }

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
    ) {
        self.fallback.violin(ctx, formatter, all_curves);
    }

    fn t_test(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.fallback.t_test(ctx, data);
    }

    fn wait(&mut self) {
        self.backend.wait();
        self.fallback.wait();
    }
}
