use criterion_plot::prelude::*;
use std::iter;
use std::path::{Path, PathBuf};
use std::process::Child;
mod pdf;
mod regression;
mod summary;
mod t_test;
use self::pdf::*;
use self::regression::*;
use self::summary::*;
use self::t_test::*;
use super::{
    FilledCurve as FilledArea, Line, LineCurve, PlotContext, PlotData, Plotter, PlottingBackend,
    Points as PointPlot,
};
use crate::estimate::Statistic;
use crate::format;
use crate::model::Benchmark;
use crate::plot::Size;
use crate::report::{BenchmarkId, ValueType};
use crate::stats::bivariate::Data;
use crate::stats::univariate::Sample;
use crate::value_formatter::ValueFormatter;

fn gnuplot_escape(string: &str) -> String {
    string.replace("_", "\\_").replace("'", "''")
}

static DEFAULT_FONT: &str = "Helvetica";
static SIZE: Size = Size(1280, 720);

const LINEWIDTH: LineWidth = LineWidth(2.);
const POINT_SIZE: PointSize = PointSize(0.75);

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);
const DARK_ORANGE: Color = Color::Rgb(255, 127, 0);
const DARK_RED: Color = Color::Rgb(227, 26, 28);

fn debug_script(path: &Path, figure: &Figure) {
    if crate::debug_enabled() {
        let script_path = path.with_extension("gnuplot");
        info!("Writing gnuplot script to {:?}", script_path);
        let result = figure.save(&script_path);
        if let Err(e) = result {
            error!("Failed to write debug output: {}", e);
        }
    }
}

/// Private
trait Append<T> {
    /// Private
    fn append_(self, item: T) -> Self;
}

// NB I wish this was in the standard library
impl<T> Append<T> for Vec<T> {
    fn append_(mut self, item: T) -> Vec<T> {
        self.push(item);
        self
    }
}

impl From<Size> for criterion_plot::Size {
    fn from(other: Size) -> Self {
        let Size(width, height) = other;
        Self(width, height)
    }
}

#[derive(Default)]
pub struct Gnuplot {
    process_list: Vec<Child>,
}
impl Gnuplot {
    pub fn new() -> Gnuplot {
        Gnuplot {
            process_list: vec![],
        }
    }
}

impl Plotter for Gnuplot {
    fn pdf(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(pdf(
            ctx.id,
            data.formatter,
            data.measurements,
            ctx.size,
            ctx.context.report_path(ctx.id, "pdf.svg"),
        ));
    }
    fn pdf_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(pdf_small(
            data.formatter,
            data.measurements,
            ctx.size,
            ctx.context.report_path(ctx.id, "pdf_small.svg"),
        ));
    }
    fn pdf_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(pdf_comparison(
            ctx.id,
            data.formatter,
            data.measurements,
            data.comparison
                .expect("Shouldn't call a comparison method without comparison data"),
            ctx.size,
            ctx.context.report_path(ctx.id, "both/pdf.svg"),
        ));
    }
    fn pdf_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(pdf_comparison_small(
            data.formatter,
            data.measurements,
            data.comparison
                .expect("Shouldn't call a comparison method without comparison data"),
            ctx.size,
            ctx.context.report_path(ctx.id, "relative_pdf_small.svg"),
        ));
    }

    fn iteration_times_comparison(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn iteration_times_comparison_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn iteration_times_thumbnail(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn iteration_times(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(regression(
            ctx.id,
            data.formatter,
            data.measurements,
            ctx.size,
            ctx.context.report_path(ctx.id, "regression.svg"),
        ))
    }
    fn regression_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        self.process_list.push(regression_small(
            data.formatter,
            data.measurements,
            ctx.size,
            ctx.context.report_path(ctx.id, "regression_small.svg"),
        ))
    }
    fn regression_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let cmp = data
            .comparison
            .expect("Shouldn't call comparison method without comparison data.");
        let base_data = Data::new(&cmp.base_iter_counts, &cmp.base_sample_times);
        self.process_list.push(regression_comparison(
            ctx.id,
            data.formatter,
            data.measurements,
            cmp,
            &base_data,
            ctx.size,
            ctx.context.report_path(ctx.id, "both/regression.svg"),
        ))
    }
    fn regression_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let cmp = data
            .comparison
            .expect("Shouldn't call comparison method without comparison data.");
        let base_data = Data::new(&cmp.base_iter_counts, &cmp.base_sample_times);
        self.process_list.push(regression_comparison_small(
            data.formatter,
            data.measurements,
            cmp,
            &base_data,
            ctx.size,
            ctx.context
                .report_path(ctx.id, "relative_regression_small.svg"),
        ))
    }

    fn abs_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn rel_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn t_test(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        if let Some(cmp) = data.comparison {
            self.process_list.push(t_test(
                ctx.id,
                cmp,
                ctx.size,
                ctx.context.report_path(ctx.id, "change/t-test.svg"),
            ));
        } else {
            error!("Comparison data is not provided for t_test plot");
        }
    }

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_benchmarks: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    ) {
        let path = ctx.line_comparison_path();
        self.process_list.push(line_comparison(
            formatter,
            ctx.id.as_title(),
            all_benchmarks,
            path,
            value_type,
            ctx.context.plot_config.summary_scale,
        ));
    }

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
    ) {
        let violin_path = ctx.violin_path();

        self.process_list.push(violin(
            formatter,
            ctx.id.as_title(),
            all_curves,
            violin_path,
            ctx.context.plot_config.summary_scale,
        ));
    }

    fn wait(&mut self) {
        let start = std::time::Instant::now();
        let child_count = self.process_list.len();
        for child in self.process_list.drain(..) {
            match child.wait_with_output() {
                Ok(ref out) if out.status.success() => {}
                Ok(out) => error!("Error in Gnuplot: {}", String::from_utf8_lossy(&out.stderr)),
                Err(e) => error!("Got IO error while waiting for Gnuplot to complete: {}", e),
            }
        }
        let elapsed = &start.elapsed();
        info!(
            "Waiting for {} gnuplot processes took {}",
            child_count,
            format::time(crate::DurationExt::to_nanos(elapsed) as f64)
        );
    }
}
impl PlottingBackend for Gnuplot {
    fn abs_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        x_unit: &str,
        distribution_curve: LineCurve,
        bootstrap_area: FilledArea,
        point_estimate: Line,
    ) {
        let xs_sample = Sample::new(distribution_curve.xs);

        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .set(Title(format!(
                "{}: {}",
                gnuplot_escape(id.as_title()),
                statistic
            )))
            .configure(Axis::BottomX, |a| {
                a.set(Label(format!("Average time ({})", x_unit)))
                    .set(Range::Limits(xs_sample.min(), xs_sample.max()))
            })
            .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .plot(
                Lines {
                    x: distribution_curve.xs,
                    y: distribution_curve.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Bootstrap distribution"))
                        .set(LineType::Solid)
                },
            )
            .plot(
                FilledCurve {
                    x: bootstrap_area.xs,
                    y1: bootstrap_area.ys_1,
                    y2: bootstrap_area.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Confidence interval"))
                        .set(Opacity(0.25))
                },
            )
            .plot(
                Lines {
                    x: &[point_estimate.start.x, point_estimate.end.x],
                    y: &[point_estimate.start.y, point_estimate.end.y],
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Point estimate"))
                        .set(LineType::Dash)
                },
            );

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap());
    }

    fn rel_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        distribution_curve: LineCurve,
        confidence_interval: FilledArea,
        point_estimate: Line,
        noise_threshold: FilledArea,
    ) {
        let xs_ = Sample::new(&distribution_curve.xs);
        let x_min = xs_.min();
        let x_max = xs_.max();

        let mut figure = Figure::new();

        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
            .configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Outside(Vertical::Top, Horizontal::Right))
            })
            .set(Title(format!(
                "{}: {}",
                gnuplot_escape(id.as_title()),
                statistic
            )))
            .configure(Axis::BottomX, |a| {
                a.set(Label("Relative change (%)"))
                    .set(Range::Limits(x_min * 100., x_max * 100.))
                    .set(ScaleFactor(100.))
            })
            .plot(
                Lines {
                    x: distribution_curve.xs,
                    y: distribution_curve.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Bootstrap distribution"))
                        .set(LineType::Solid)
                },
            )
            .plot(
                FilledCurve {
                    x: confidence_interval.xs,
                    y1: confidence_interval.ys_1,
                    y2: confidence_interval.ys_2,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Confidence interval"))
                        .set(Opacity(0.25))
                },
            )
            .plot(
                Lines {
                    x: &[point_estimate.start.x, point_estimate.end.x],
                    y: &[point_estimate.start.y, point_estimate.end.y],
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(LINEWIDTH)
                        .set(Label("Point estimate"))
                        .set(LineType::Dash)
                },
            )
            .plot(
                FilledCurve {
                    x: noise_threshold.xs,
                    y1: noise_threshold.ys_1,
                    y2: noise_threshold.ys_2,
                },
                |c| {
                    c.set(Axes::BottomXRightY)
                        .set(DARK_RED)
                        .set(Label("Noise threshold"))
                        .set(Opacity(0.1))
                },
            );

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn iteration_times(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        file_path: PathBuf,

        unit: &str,
        is_thumbnail: bool,
        current_times: PointPlot,
        base_times: Option<PointPlot>,
    ) {
        let mut figure = Figure::new();
        figure
            .set(Font(DEFAULT_FONT))
            .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
            .configure(Axis::BottomX, |a| {
                a.configure(Grid::Major, |g| g.show()).set(Label("Sample"))
            })
            .configure(Axis::LeftY, |a| {
                a.configure(Grid::Major, |g| g.show())
                    .set(Label(format!("Average Iteration Time ({})", unit)))
            })
            .plot(
                Points {
                    x: current_times.xs,
                    y: current_times.ys,
                },
                |c| {
                    c.set(DARK_BLUE)
                        .set(Label("Current"))
                        .set(PointSize(0.5))
                        .set(PointType::FilledCircle)
                },
            );

        if let Some(base_times) = base_times {
            figure.plot(
                Points {
                    x: base_times.xs,
                    y: base_times.ys,
                },
                |c| {
                    c.set(DARK_RED)
                        .set(Label("Base"))
                        .set(PointSize(0.5))
                        .set(PointType::FilledCircle)
                },
            );
        }

        if !is_thumbnail {
            figure.set(Title(gnuplot_escape(id.as_title())));
            figure.configure(Key, |k| {
                k.set(Justification::Left)
                    .set(Order::SampleText)
                    .set(Position::Inside(Vertical::Top, Horizontal::Left))
            });
        } else {
            figure.configure(Key, |k| k.hide());
        }

        debug_script(&file_path, &figure);
        self.process_list
            .push(figure.set(Output(file_path)).draw().unwrap())
    }

    fn wait(&mut self) {
        Plotter::wait(self)
    }
}
