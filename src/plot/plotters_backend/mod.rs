use crate::estimate::Statistic;
use crate::kde;
use crate::model::Benchmark;
use crate::plot::{
    FilledCurve, Line, LineCurve, PlotContext, PlotData, Plotter, PlottingBackend, ReportContext,
    Size,
};
use crate::report::{BenchmarkId, ComparisonData, MeasurementData, ValueType};
use crate::stats::bivariate::Data;
use crate::stats::univariate::Sample;
use crate::value_formatter::ValueFormatter;
use plotters::data::float::pretty_print_float;
use plotters::prelude::*;

static DEFAULT_FONT: FontFamily = FontFamily::SansSerif;
static SIZE: Size = Size(960, 540);
static POINT_SIZE: u32 = 3;

const DARK_BLUE: RGBColor = RGBColor(31, 120, 180);
const DARK_ORANGE: RGBColor = RGBColor(255, 127, 0);
const DARK_RED: RGBColor = RGBColor(227, 26, 28);

mod distributions;
mod iteration_times;
mod pdf;
mod regression;
mod summary;
mod t_test;

impl From<Size> for (u32, u32) {
    fn from(other: Size) -> Self {
        let Size(width, height) = other;
        (width as u32, height as u32)
    }
}

#[derive(Default)]
pub struct PlottersBackend;

#[allow(unused_variables)]
impl Plotter for PlottersBackend {
    fn pdf(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        if let Some(cmp) = data.comparison {
            let (path, title) = if ctx.is_thumbnail {
                (
                    ctx.context.report_path(ctx.id, "relative_pdf_small.svg"),
                    None,
                )
            } else {
                (
                    ctx.context.report_path(ctx.id, "both/pdf.svg"),
                    Some(ctx.id.as_title()),
                )
            };
            pdf::pdf_comparison_figure(
                path.as_ref(),
                title,
                data.formatter,
                data.measurements,
                cmp,
                ctx.size,
            );
            return;
        }
        if ctx.is_thumbnail {
            pdf::pdf_small(
                ctx.id,
                ctx.context,
                data.formatter,
                data.measurements,
                ctx.size,
            );
        } else {
            pdf::pdf(
                ctx.id,
                ctx.context,
                data.formatter,
                data.measurements,
                ctx.size,
            );
        }
    }

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let (title, path) = match (data.comparison.is_some(), ctx.is_thumbnail) {
            (true, true) => (
                None,
                ctx.context
                    .report_path(ctx.id, "relative_regression_small.svg"),
            ),
            (true, false) => (
                Some(ctx.id.as_title()),
                ctx.context.report_path(ctx.id, "both/regression.svg"),
            ),
            (false, true) => (
                None,
                ctx.context.report_path(ctx.id, "regression_small.svg"),
            ),
            (false, false) => (
                Some(ctx.id.as_title()),
                ctx.context.report_path(ctx.id, "regression.svg"),
            ),
        };

        if let Some(cmp) = data.comparison {
            let base_data = Data::new(&cmp.base_iter_counts, &cmp.base_sample_times);
            regression::regression_comparison_figure(
                title,
                path.as_path(),
                data.formatter,
                data.measurements,
                cmp,
                &base_data,
                ctx.size,
            );
        } else {
            regression::regression_figure(
                title,
                path.as_path(),
                data.formatter,
                data.measurements,
                ctx.size,
            );
        }
    }

    fn iteration_times(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let (title, path) = match (data.comparison.is_some(), ctx.is_thumbnail) {
            (true, true) => (
                None,
                ctx.context
                    .report_path(ctx.id, "relative_iteration_times_small.svg"),
            ),
            (true, false) => (
                Some(ctx.id.as_title()),
                ctx.context.report_path(ctx.id, "both/iteration_times.svg"),
            ),
            (false, true) => (
                None,
                ctx.context.report_path(ctx.id, "iteration_times_small.svg"),
            ),
            (false, false) => (
                Some(ctx.id.as_title()),
                ctx.context.report_path(ctx.id, "iteration_times.svg"),
            ),
        };

        if let Some(cmp) = data.comparison {
            let base_data = Data::new(&cmp.base_iter_counts, &cmp.base_sample_times);
            iteration_times::iteration_times_comparison_figure(
                title,
                path.as_path(),
                data.formatter,
                data.measurements,
                cmp,
                ctx.size,
            );
        } else {
            iteration_times::iteration_times_figure(
                title,
                path.as_path(),
                data.formatter,
                data.measurements,
                ctx.size,
            );
        }
    }

    fn abs_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }

    fn rel_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        distributions::rel_distributions(
            ctx.id,
            ctx.context,
            data.measurements,
            data.comparison.unwrap(),
            ctx.size,
        );
    }

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_benchmarks: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    ) {
        let path = ctx.line_comparison_path();
        summary::line_comparison(
            formatter,
            ctx.id.as_title(),
            all_benchmarks,
            &path,
            value_type,
            ctx.context.plot_config.summary_scale,
        );
    }

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_benchmarks: &[(&BenchmarkId, &Benchmark)],
    ) {
        let violin_path = ctx.violin_path();

        summary::violin(
            formatter,
            ctx.id.as_title(),
            all_benchmarks,
            &violin_path,
            ctx.context.plot_config.summary_scale,
        );
    }

    fn t_test(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        let title = ctx.id.as_title();
        let path = ctx.context.report_path(ctx.id, "change/t-test.svg");
        t_test::t_test(path.as_path(), title, data.comparison.unwrap(), ctx.size);
    }

    fn wait(&mut self) {}
}
impl PlottingBackend for PlottersBackend {
    fn abs_distribution(
        &mut self,
        id: &BenchmarkId,
        context: &ReportContext,
        statistic: Statistic,
        size: Option<Size>,

        x_unit: &str,
        distribution_curve: LineCurve,
        bootstrap_area: FilledCurve,
        point_estimate: Line,
    ) {
        let path = context.report_path(id, &format!("{}.svg", statistic));
        let root_area = SVGBackend::new(&path, size.unwrap_or(SIZE).into()).into_drawing_area();

        let x_range = plotters::data::fitting_range(distribution_curve.xs.iter());
        let mut y_range = plotters::data::fitting_range(distribution_curve.ys.iter());

        y_range.end *= 1.1;

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .caption(
                format!("{}:{}", id.as_title(), statistic),
                (DEFAULT_FONT, 20),
            )
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range)
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .x_desc(format!("Average time ({})", x_unit))
            .y_desc("Density (a.u.)")
            .x_label_formatter(&|&v| pretty_print_float(v, true))
            .y_label_formatter(&|&v| pretty_print_float(v, true))
            .draw()
            .unwrap();

        chart
            .draw_series(LineSeries::new(
                distribution_curve
                    .xs
                    .iter()
                    .copied()
                    .zip(distribution_curve.ys.iter().copied()),
                &DARK_BLUE,
            ))
            .unwrap()
            .label("Bootstrap distribution")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(AreaSeries::new(
                (bootstrap_area.xs.iter().copied()).zip(bootstrap_area.ys_1.iter().copied()),
                0.0,
                DARK_BLUE.mix(0.25).filled().stroke_width(3),
            ))
            .unwrap()
            .label("Confidence interval")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.25).filled())
            });

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![
                    (point_estimate.start.x, point_estimate.start.y),
                    (point_estimate.end.x, point_estimate.end.y),
                ],
                DARK_BLUE.filled().stroke_width(3),
            )))
            .unwrap()
            .label("Point estimate")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .draw()
            .unwrap();
    }

    fn wait(&mut self) {
        Plotter::wait(self)
    }
}
