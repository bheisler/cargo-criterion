use crate::connection::AxisScale;
use crate::estimate::Statistic;
use crate::plot::{
    FilledCurve, Line, LineCurve, PlottingBackend, Points, Rectangle as RectangleArea, Size,
    VerticalLine,
};
use crate::report::{BenchmarkId, ValueType};
use crate::stats::univariate::Sample;
use plotters::coord::{AsRangedCoord, Shift};
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

const NUM_COLORS: usize = 8;
static COMPARISON_COLORS: [RGBColor; NUM_COLORS] = [
    RGBColor(178, 34, 34),
    RGBColor(46, 139, 87),
    RGBColor(0, 139, 139),
    RGBColor(255, 215, 0),
    RGBColor(0, 0, 139),
    RGBColor(220, 20, 60),
    RGBColor(139, 0, 139),
    RGBColor(0, 255, 127),
];

impl From<Size> for (u32, u32) {
    fn from(other: Size) -> Self {
        let Size(width, height) = other;
        (width as u32, height as u32)
    }
}

impl VerticalLine {
    fn to_line_vec(&self, y_max: f64) -> Vec<(f64, f64)> {
        vec![(self.x, 0.0), (self.x, y_max)]
    }
}
impl Line {
    fn to_line_vec(&self) -> Vec<(f64, f64)> {
        vec![(self.start.x, self.start.y), (self.end.x, self.end.y)]
    }
}
impl<'a> LineCurve<'a> {
    fn to_points(&self) -> impl Iterator<Item = (f64, f64)> + 'a {
        (self.xs.iter().copied()).zip(self.ys.iter().copied())
    }
}
impl<'a> FilledCurve<'a> {
    fn to_points(&self) -> impl Iterator<Item = (f64, f64)> + 'a {
        (self.xs.iter().copied()).zip(self.ys_1.iter().copied())
    }
}
impl<'a> Points<'a> {
    fn to_points(&self) -> impl Iterator<Item = (f64, f64)> + 'a {
        (self.xs.iter().copied()).zip(self.ys.iter().copied())
    }
}

#[derive(Default)]
pub struct PlottersBackend;
impl PlottersBackend {
    fn draw_line_comparison_figure<
        XR: AsRangedCoord<Value = f64>,
        YR: AsRangedCoord<Value = f64>,
    >(
        &self,
        root_area: DrawingArea<SVGBackend, Shift>,
        y_unit: &str,
        x_range: XR,
        y_range: YR,
        value_type: ValueType,
        data: &[(Option<&String>, LineCurve)],
    ) {
        let input_suffix = match value_type {
            ValueType::Bytes => " Size (Bytes)",
            ValueType::Elements => " Size (Elements)",
            ValueType::Value => "",
        };

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range)
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .x_desc(format!("Input{}", input_suffix))
            .y_desc(format!("Average time ({})", y_unit))
            .draw()
            .unwrap();

        for (id, (name, curve)) in (0..).zip(data.into_iter()) {
            let series = chart
                .draw_series(
                    LineSeries::new(
                        curve.to_points(),
                        COMPARISON_COLORS[id % NUM_COLORS].filled(),
                    )
                    .point_size(POINT_SIZE),
                )
                .unwrap();
            if let Some(name) = name {
                let name: &str = &*name;
                series.label(name).legend(move |(x, y)| {
                    Rectangle::new(
                        [(x, y - 5), (x + 20, y + 5)],
                        COMPARISON_COLORS[id % NUM_COLORS].filled(),
                    )
                });
            }
        }

        chart
            .configure_series_labels()
            .position(SeriesLabelPosition::UpperLeft)
            .draw()
            .unwrap();
    }

    fn draw_violin_figure<XR: AsRangedCoord<Value = f64>, YR: AsRangedCoord<Value = f64>>(
        &self,
        root_area: DrawingArea<SVGBackend, Shift>,
        unit: &str,
        x_range: XR,
        y_range: YR,
        data: &[(&str, LineCurve)],
    ) {
        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .set_label_area_size(LabelAreaPosition::Left, (10).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_width().min(40))
            .build_ranged(x_range, y_range)
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .y_desc("Input")
            .x_desc(format!("Average time ({})", unit))
            .y_label_style((DEFAULT_FONT, 10))
            .y_label_formatter(&|v: &f64| data[v.round() as usize].0.to_string())
            .y_labels(data.len())
            .draw()
            .unwrap();

        for (i, (_, curve)) in data.into_iter().enumerate() {
            let base = i as f64;

            chart
                .draw_series(AreaSeries::new(
                    curve.to_points().map(|(x, y)| (x, base + y / 2.0)),
                    base,
                    &DARK_BLUE.mix(0.25),
                ))
                .unwrap();

            chart
                .draw_series(AreaSeries::new(
                    curve.to_points().map(|(x, y)| (x, base - y / 2.0)),
                    base,
                    &DARK_BLUE.mix(0.25),
                ))
                .unwrap();
        }
    }
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
            .draw_series(LineSeries::new(distribution_curve.to_points(), &DARK_BLUE))
            .unwrap()
            .label("Bootstrap distribution")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(AreaSeries::new(
                bootstrap_area.to_points(),
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
                point_estimate.to_line_vec(),
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
            .draw_series(LineSeries::new(distribution_curve.to_points(), &DARK_BLUE))
            .unwrap()
            .label("Bootstrap distribution")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(AreaSeries::new(
                confidence_interval.to_points(),
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
                point_estimate.to_line_vec(),
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
                (current_times.to_points())
                    .map(|(x, y)| Circle::new((x, y), POINT_SIZE, DARK_BLUE.filled())),
            )
            .unwrap()
            .label("Current")
            .legend(|(x, y)| Circle::new((x + 10, y), POINT_SIZE, DARK_BLUE.filled()));

        if let Some(base_times) = base_times {
            chart
                .draw_series(
                    (base_times.to_points())
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
                (sample.to_points())
                    .map(|(x, y)| Circle::new((x, y), POINT_SIZE, DARK_BLUE.filled())),
            )
            .unwrap()
            .label("Sample")
            .legend(|(x, y)| Circle::new((x + 10, y), POINT_SIZE, DARK_BLUE.filled()));

        chart
            .draw_series(std::iter::once(PathElement::new(
                regression.to_line_vec(),
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
                PathElement::new(base_regression.to_line_vec(), &DARK_RED).into_dyn(),
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
                PathElement::new(current_regression.to_line_vec(), &DARK_BLUE).into_dyn(),
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
        max_iters: f64,
        pdf: FilledCurve,
        mean: VerticalLine,
        fences: (VerticalLine, VerticalLine, VerticalLine, VerticalLine),
        points: (Points, Points, Points),
    ) {
        let (low_severe, low_mild, high_mild, high_severe) = fences;
        let (not_outlier, mild, severe) = points;
        let xs_ = Sample::new(&pdf.xs);

        let size = size.unwrap_or(SIZE);
        let root_area = SVGBackend::new(&path, size.into()).into_drawing_area();

        let range = plotters::data::fitting_range(pdf.ys_1.iter());

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
                pdf.to_points(),
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
                mean.to_line_vec(max_iters),
                &DARK_BLUE,
            )))
            .unwrap()
            .label("Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart
            .draw_series(vec![
                PathElement::new(low_mild.to_line_vec(max_iters), &DARK_ORANGE),
                PathElement::new(high_mild.to_line_vec(max_iters), &DARK_ORANGE),
                PathElement::new(low_severe.to_line_vec(max_iters), &DARK_RED),
                PathElement::new(high_severe.to_line_vec(max_iters), &DARK_RED),
            ])
            .unwrap();

        let mut draw_data_point_series = |points: Points, color: RGBAColor, name: &str| {
            chart
                .draw_series(
                    (points.to_points())
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
                pdf.to_points(),
                0.0,
                DARK_BLUE.mix(0.25).filled(),
            ))
            .unwrap();

        chart
            .draw_series(std::iter::once(PathElement::new(
                mean.to_line_vec(),
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
                base_pdf.to_points(),
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
                current_pdf.to_points(),
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
                base_mean.to_line_vec(),
                DARK_RED.filled().stroke_width(2),
            )))
            .unwrap()
            .label("Base Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_RED));

        chart
            .draw_series(std::iter::once(PathElement::new(
                current_mean.to_line_vec(),
                DARK_BLUE.filled().stroke_width(2),
            )))
            .unwrap()
            .label("New Mean")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        if !is_thumbnail {
            chart.configure_series_labels().draw().unwrap();
        }
    }

    fn t_test(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        t: VerticalLine,
        t_distribution: FilledCurve,
    ) {
        let x_range = plotters::data::fitting_range(t_distribution.xs.iter());
        let mut y_range = plotters::data::fitting_range(t_distribution.ys_1.iter());
        y_range.start = 0.0;
        y_range.end *= 1.1;

        let root_area = SVGBackend::new(&path, size.unwrap_or(SIZE).into()).into_drawing_area();

        let mut chart = ChartBuilder::on(&root_area)
            .margin((5).percent())
            .caption(
                format!("{}: Welch t test", id.as_title()),
                (DEFAULT_FONT, 20),
            )
            .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
            .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
            .build_ranged(x_range, y_range.clone())
            .unwrap();

        chart
            .configure_mesh()
            .disable_mesh()
            .y_desc("Density")
            .x_desc("t score")
            .draw()
            .unwrap();

        chart
            .draw_series(AreaSeries::new(
                t_distribution.to_points(),
                0.0,
                &DARK_BLUE.mix(0.25),
            ))
            .unwrap()
            .label("t distribution")
            .legend(|(x, y)| {
                Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.25).filled())
            });

        chart
            .draw_series(std::iter::once(PathElement::new(
                t.to_line_vec(y_range.end),
                DARK_BLUE.filled().stroke_width(2),
            )))
            .unwrap()
            .label("t statistic")
            .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

        chart.configure_series_labels().draw().unwrap();
    }

    fn line_comparison(
        &mut self,
        path: PathBuf,
        title: &str,
        unit: &str,
        value_type: ValueType,
        axis_scale: AxisScale,
        lines: &[(Option<&String>, LineCurve)],
    ) {
        let x_range =
            plotters::data::fitting_range(lines.iter().flat_map(|(_, curve)| curve.xs.iter()));
        let y_range =
            plotters::data::fitting_range(lines.iter().flat_map(|(_, curve)| curve.ys.iter()));
        let root_area = SVGBackend::new(&path, SIZE.into())
            .into_drawing_area()
            .titled(&format!("{}: Comparison", title), (DEFAULT_FONT, 20))
            .unwrap();

        match axis_scale {
            AxisScale::Linear => self
                .draw_line_comparison_figure(root_area, &unit, x_range, y_range, value_type, lines),
            AxisScale::Logarithmic => self.draw_line_comparison_figure(
                root_area,
                &unit,
                LogRange(x_range),
                LogRange(y_range),
                value_type,
                lines,
            ),
        }
    }

    fn violin(
        &mut self,
        path: PathBuf,
        title: &str,
        unit: &str,
        axis_scale: AxisScale,
        lines: &[(&str, LineCurve)],
    ) {
        let x_range =
            plotters::data::fitting_range(lines.iter().flat_map(|(_, curve)| curve.xs.iter()));
        let y_range = -0.5..lines.len() as f64 - 0.5;

        let size = (960, 150 + (18 * lines.len() as u32));

        let root_area = SVGBackend::new(&path, size)
            .into_drawing_area()
            .titled(&format!("{}: Violin plot", title), (DEFAULT_FONT, 20))
            .unwrap();

        match axis_scale {
            AxisScale::Linear => self.draw_violin_figure(root_area, &unit, x_range, y_range, lines),
            AxisScale::Logarithmic => {
                self.draw_violin_figure(root_area, &unit, LogRange(x_range), y_range, lines)
            }
        }
    }

    fn wait(&mut self) {}
}
