use crate::estimate::Statistic;
use crate::kde;
use crate::model::Benchmark;
use crate::plot::{
    FilledCurve, Line, LineCurve, PlotContext, PlotData, Plotter, PlottingBackend, Points,
    Rectangle as RectangleArea, Size,
};
use crate::report::{BenchmarkId, ValueType};
use crate::stats::univariate::Sample;
use crate::value_formatter::ValueFormatter;
use plotters::data::float::pretty_print_float;
use plotters::prelude::*;
use plotters::style::RGBAColor;
use std::path::PathBuf;

static DEFAULT_FONT: FontFamily = FontFamily::SansSerif;
static SIZE: Size = Size(960, 540);
static POINT_SIZE: u32 = 3;

const DARK_BLUE: RGBColor = RGBColor(31, 120, 180);
const DARK_ORANGE: RGBColor = RGBColor(255, 127, 0);
const DARK_RED: RGBColor = RGBColor(227, 26, 28);

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
        unimplemented!()
    }
    fn pdf_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn pdf_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn pdf_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn regression_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }

    fn iteration_times(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times_comparison(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }
    fn iteration_times_comparison_thumbnail(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>) {
        unimplemented!()
    }

    fn abs_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
    }

    fn rel_distributions(&mut self, _: PlotContext<'_>, _: PlotData<'_>) {
        unimplemented!()
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
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        x_unit: &str,
        distribution_curve: LineCurve,
        bootstrap_area: FilledCurve,
        point_estimate: Line,
    ) {
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

    fn rel_distribution(
        &mut self,
        id: &BenchmarkId,
        statistic: Statistic,
        size: Option<Size>,
        path: PathBuf,

        distribution_curve: LineCurve,
        confidence_interval: FilledCurve,
        point_estimate: Line,
        noise_threshold: RectangleArea,
    ) {
        let xs_ = Sample::new(&distribution_curve.xs);
        let x_min = xs_.min();
        let x_max = xs_.max();

        let y_range = plotters::data::fitting_range(distribution_curve.ys);
        let root_area = SVGBackend::new(&path, size.unwrap_or(SIZE).into()).into_drawing_area();

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .caption(
                format!("{}:{}", id.as_title(), statistic),
                (DEFAULT_FONT, 20),
            )
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_min..x_max, y_range.clone())
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .x_desc("Relative change (%)")
            .y_desc("Density (a.u.)")
            .x_label_formatter(&|&v| pretty_print_float(v, true))
            .y_label_formatter(&|&v| pretty_print_float(v, true))
            .draw()
            .unwrap();

        chart
            .draw_series(LineSeries::new(
                (distribution_curve.xs.iter().copied()).zip(distribution_curve.ys.iter().copied()),
                &DARK_BLUE,
            ))
            .unwrap()
            .label("Bootstrap distribution")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(AreaSeries::new(
                (confidence_interval.xs.iter().copied())
                    .zip(confidence_interval.ys_1.iter().copied()),
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
            .draw_series(std::iter::once(Rectangle::new(
                [
                    (noise_threshold.left, y_range.start),
                    (noise_threshold.right, y_range.end),
                ],
                DARK_RED.mix(0.1).filled(),
            )))
            .unwrap()
            .label("Noise threshold")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_RED.mix(0.25).filled())
            });
        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperRight)
            .draw()
            .unwrap();
    }

    fn iteration_times(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        is_thumbnail: bool,
        current_times: Points,
        base_times: Option<Points>,
    ) {
        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let mut cb = ChartBuilder::on(&root_area);

        let (x_range, y_range) = if let Some(base) = &base_times {
            let max_x = Sample::new(current_times.xs)
                .max()
                .max(Sample::new(base.xs).max());
            let x_range = (1.0)..(max_x);
            let y_range =
                plotters::data::fitting_range(current_times.ys.iter().chain(base.ys.iter()));
            (x_range, y_range)
        } else {
            let max_x = Sample::new(current_times.xs).max();
            let x_range = (1.0)..(max_x);
            let y_range = plotters::data::fitting_range(current_times.ys.iter());
            (x_range, y_range)
        };

        let mut chart = cb
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range)
            .unwrap();

        chart
            .configure_mesh()
            .y_desc(format!("Average Iteration Time ({})", unit))
            .x_label_formatter(&|x| pretty_print_float(*x, true))
            .line_style_2(&TRANSPARENT)
            .draw()
            .unwrap();

        chart
            .draw_series(
                (current_times.xs.iter().copied())
                    .zip(current_times.ys.iter().copied())
                    .map(|(x, y)| Circle::new((x, y), POINT_SIZE, DARK_BLUE.filled())),
            )
            .unwrap()
            .label("Current")
            .legend(|(x, y)| Circle::new((x + 10, y), POINT_SIZE, DARK_BLUE.filled()));

        if let Some(base_times) = base_times {
            chart
                .draw_series(
                    (base_times.xs.iter().copied())
                        .zip(base_times.ys.iter().copied())
                        .map(|(x, y)| Circle::new((x, y), POINT_SIZE, DARK_RED.filled())),
                )
                .unwrap()
                .label("Base")
                .legend(|(x, y)| Circle::new((x + 10, y), POINT_SIZE, DARK_RED.filled()));
        }

        if !is_thumbnail {
            cb.caption(id.as_title(), (DEFAULT_FONT, 20));
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::UpperLeft)
                .draw()
                .unwrap();
        }
    }

    fn regression(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        x_label: &str,
        x_scale: f64,
        unit: &str,
        sample: Points,
        regression: Line,
        confidence_interval: FilledCurve,
    ) {
        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let mut cb = ChartBuilder::on(&root_area);
        if !is_thumbnail {
            cb.caption(id.as_title(), (DEFAULT_FONT, 20));
        }

        let x_range = plotters::data::fitting_range(sample.xs.iter());
        let y_range = plotters::data::fitting_range(sample.ys.iter());

        let mut chart = cb
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range)
            .unwrap();

        chart
            .configure_mesh()
            .x_desc(x_label)
            .y_desc(format!("Total sample time ({})", unit))
            .x_label_formatter(&|x| pretty_print_float(x * x_scale, true))
            .line_style_2(&TRANSPARENT)
            .draw()
            .unwrap();

        chart
            .draw_series(
                (sample.xs.iter().copied())
                    .zip(sample.ys.iter().copied())
                    .map(|(x, y)| Circle::new((x, y), POINT_SIZE, DARK_BLUE.filled())),
            )
            .unwrap()
            .label("Sample")
            .legend(|(x, y)| Circle::new((x + 10, y), POINT_SIZE, DARK_BLUE.filled()));

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![
                    (regression.start.x, regression.start.y),
                    (regression.end.x, regression.end.y),
                ],
                &DARK_BLUE,
            )))
            .unwrap()
            .label("Linear regression")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    DARK_BLUE.filled().stroke_width(2),
                )
            });

        chart
            .draw_series(std::iter::once(Polygon::new(
                vec![
                    (confidence_interval.xs[0], confidence_interval.ys_2[0]),
                    (confidence_interval.xs[1], confidence_interval.ys_1[1]),
                    (confidence_interval.xs[1], confidence_interval.ys_2[1]),
                ],
                DARK_BLUE.mix(0.25).filled(),
            )))
            .unwrap()
            .label("Confidence interval")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.25).filled())
            });

        if !is_thumbnail {
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::UpperLeft)
                .draw()
                .unwrap();
        }
    }

    fn regression_comparison(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        x_label: &str,
        x_scale: f64,
        unit: &str,
        current_regression: Line,
        current_confidence_interval: FilledCurve,
        base_regression: Line,
        base_confidence_interval: FilledCurve,
    ) {
        let y_max = current_regression.end.y.max(base_regression.end.y);
        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let mut cb = ChartBuilder::on(&root_area);
        if !is_thumbnail {
            cb.caption(id.as_title(), (DEFAULT_FONT, 20));
        }

        let mut chart = cb
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(0.0..current_regression.end.x, 0.0..y_max)
            .unwrap();

        chart
            .configure_mesh()
            .x_desc(x_label)
            .y_desc(format!("Total sample time ({})", unit))
            .x_label_formatter(&|x| pretty_print_float(x * x_scale, true))
            .line_style_2(&TRANSPARENT)
            .draw()
            .unwrap();

        chart
            .draw_series(vec![
                PathElement::new(
                    vec![
                        (base_regression.start.x, base_regression.start.y),
                        (base_regression.end.x, base_regression.end.y),
                    ],
                    &DARK_RED,
                )
                .into_dyn(),
                Polygon::new(
                    vec![
                        (
                            base_confidence_interval.xs[0],
                            base_confidence_interval.ys_2[0],
                        ),
                        (
                            base_confidence_interval.xs[1],
                            base_confidence_interval.ys_1[1],
                        ),
                        (
                            base_confidence_interval.xs[1],
                            base_confidence_interval.ys_2[1],
                        ),
                    ],
                    DARK_RED.mix(0.25).filled(),
                )
                .into_dyn(),
            ])
            .unwrap()
            .label("Base Sample")
            .legend(|(x, y)| {
                PathElement::new(vec![(x, y), (x + 20, y)], DARK_RED.filled().stroke_width(2))
            });

        chart
            .draw_series(vec![
                PathElement::new(
                    vec![
                        (current_regression.start.x, current_regression.start.y),
                        (current_regression.end.x, current_regression.end.y),
                    ],
                    &DARK_BLUE,
                )
                .into_dyn(),
                Polygon::new(
                    vec![
                        (
                            current_confidence_interval.xs[0],
                            current_confidence_interval.ys_2[0],
                        ),
                        (
                            current_confidence_interval.xs[1],
                            current_confidence_interval.ys_1[1],
                        ),
                        (
                            current_confidence_interval.xs[1],
                            current_confidence_interval.ys_2[1],
                        ),
                    ],
                    DARK_BLUE.mix(0.25).filled(),
                )
                .into_dyn(),
            ])
            .unwrap()
            .label("New Sample")
            .legend(|(x, y)| {
                PathElement::new(
                    vec![(x, y), (x + 20, y)],
                    DARK_BLUE.filled().stroke_width(2),
                )
            });

        if !is_thumbnail {
            chart
                .configure_series_labels()
                .position(SeriesLabelPosition::UpperLeft)
                .draw()
                .unwrap();
        }
    }

    fn pdf_full(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        y_label: &str,
        y_scale: f64,
        pdf: FilledCurve,
        mean: Line,
        fences: (Line, Line, Line, Line),
        points: (Points, Points, Points),
    ) {
        let (low_severe, low_mild, high_mild, high_severe) = fences;
        let (not_outlier, mild, severe) = points;
        let xs_ = Sample::new(&pdf.xs);

        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let range = plotters::data::fitting_range(pdf.ys_1.iter());
        let max_iters = mean.end.y;

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .caption(id.as_title(), (DEFAULT_FONT, 20))
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Right, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(xs_.min()..xs_.max(), 0.0..max_iters)
            .unwrap()
            .set_secondary_coord(xs_.min()..xs_.max(), 0.0..range.end);

        chart
            .configure_mesh()
            .disable_mesh()
            .y_desc(y_label)
            .x_desc(format!("Average Time ({})", unit))
            .x_label_formatter(&|&x| pretty_print_float(x, true))
            .y_label_formatter(&|&y| pretty_print_float(y * y_scale, true))
            .draw()
            .unwrap();

        chart
            .configure_secondary_axes()
            .y_desc("Density (a.u.)")
            .x_label_formatter(&|&x| pretty_print_float(x, true))
            .y_label_formatter(&|&y| pretty_print_float(y, true))
            .draw()
            .unwrap();

        chart
            .draw_secondary_series(AreaSeries::new(
                (pdf.xs.iter().copied()).zip(pdf.ys_1.iter().copied()),
                0.0,
                DARK_BLUE.mix(0.5).filled(),
            ))
            .unwrap()
            .label("PDF")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.5).filled())
            });

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![(mean.start.x, mean.start.y), (mean.end.x, mean.end.y)],
                &DARK_BLUE,
            )))
            .unwrap()
            .label("Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(vec![
                PathElement::new(
                    vec![
                        (low_mild.start.x, low_mild.start.y),
                        (low_mild.end.x, low_mild.end.y),
                    ],
                    &DARK_ORANGE,
                ),
                PathElement::new(
                    vec![
                        (high_mild.start.x, high_mild.start.y),
                        (high_mild.end.x, high_mild.end.y),
                    ],
                    &DARK_ORANGE,
                ),
                PathElement::new(
                    vec![
                        (low_severe.start.x, low_severe.start.y),
                        (low_severe.end.x, low_severe.end.y),
                    ],
                    &DARK_RED,
                ),
                PathElement::new(
                    vec![
                        (high_severe.start.x, high_severe.start.y),
                        (high_severe.end.x, high_severe.end.y),
                    ],
                    &DARK_RED,
                ),
            ])
            .unwrap();

        let mut draw_data_point_series = |points: Points, color: RGBAColor, name: &str| {
            chart
                .draw_series(
                    (points.xs.iter().copied())
                        .zip(points.ys.iter().copied())
                        .map(|(x, y)| Circle::new((x, y), POINT_SIZE, color.filled())),
                )
                .unwrap()
                .label(name)
                .legend(move |(x, y)| Circle::new((x + 10, y), POINT_SIZE, color.filled()));
        };

        draw_data_point_series(not_outlier, DARK_BLUE.to_rgba(), "\"Clean\" sample");
        draw_data_point_series(mild, RGBColor(255, 127, 0).to_rgba(), "Mild outliers");
        draw_data_point_series(severe, DARK_RED.to_rgba(), "Severe outliers");
        chart.configure_series_labels().draw().unwrap();
    }

    fn pdf_thumbnail(
        &mut self,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        mean: Line,
        pdf: FilledCurve,
    ) {
        let xs_ = Sample::new(pdf.xs);
        let ys_ = Sample::new(pdf.ys_1);

        let y_limit = ys_.max() * 1.1;

        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(xs_.min()..xs_.max(), 0.0..y_limit)
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .y_desc("Density (a.u.)")
            .x_desc(format!("Average Time ({})", unit))
            .x_label_formatter(&|&x| pretty_print_float(x, true))
            .y_label_formatter(&|&y| pretty_print_float(y, true))
            .x_labels(5)
            .draw()
            .unwrap();

        chart
            .draw_series(AreaSeries::new(
                (pdf.xs.iter().copied()).zip(pdf.ys_1.iter().copied()),
                0.0,
                DARK_BLUE.mix(0.25).filled(),
            ))
            .unwrap();

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![(mean.start.x, mean.start.y), (mean.end.x, mean.end.y)],
                DARK_BLUE.filled().stroke_width(2),
            )))
            .unwrap();
    }

    fn pdf_comparison(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        unit: &str,
        current_mean: Line,
        current_pdf: FilledCurve,
        base_mean: Line,
        base_pdf: FilledCurve,
    ) {
        let x_range =
            plotters::data::fitting_range(base_pdf.xs.iter().chain(current_pdf.xs.iter()));
        let y_range =
            plotters::data::fitting_range(base_pdf.ys_1.iter().chain(current_pdf.ys_1.iter()));

        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let mut cb = ChartBuilder::on(&root_area);

        if !is_thumbnail {
            cb.caption(id.as_title(), (DEFAULT_FONT, 20));
        }

        let mut chart = cb
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range.clone())
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .y_desc("Density (a.u.)")
            .x_desc(format!("Average Time ({})", unit))
            .x_label_formatter(&|&x| pretty_print_float(x, true))
            .y_label_formatter(&|&y| pretty_print_float(y, true))
            .x_labels(5)
            .draw()
            .unwrap();

        chart
            .draw_series(AreaSeries::new(
                (base_pdf.xs.iter().copied()).zip(base_pdf.ys_1.iter().copied()),
                y_range.start,
                DARK_RED.mix(0.5).filled(),
            ))
            .unwrap()
            .label("Base PDF")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_RED.mix(0.5).filled())
            });

        chart
            .draw_series(AreaSeries::new(
                (current_pdf.xs.iter().copied()).zip(current_pdf.ys_1.iter().copied()),
                y_range.start,
                DARK_BLUE.mix(0.5).filled(),
            ))
            .unwrap()
            .label("New PDF")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.5).filled())
            });

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![
                    (base_mean.start.x, base_mean.start.y),
                    (base_mean.end.x, base_mean.end.y),
                ],
                DARK_RED.filled().stroke_width(2),
            )))
            .unwrap()
            .label("Base Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_RED));

        chart
            .draw_series(std::iter::once(PathElement::new(
                vec![
                    (current_mean.start.x, current_mean.start.y),
                    (current_mean.end.x, current_mean.end.y),
                ],
                DARK_BLUE.filled().stroke_width(2),
            )))
            .unwrap()
            .label("New Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        if !is_thumbnail {
            chart.configure_series_labels().draw().unwrap();
        }
    }

    fn wait(&mut self) {
        Plotter::wait(self)
    }
}
