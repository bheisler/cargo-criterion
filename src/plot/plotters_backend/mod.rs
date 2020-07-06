use crate::connection::AxisScale;
use crate::estimate::Statistic;
use crate::plot::{
    FilledCurve, Line, LineCurve, PlottingBackend, Points, Rectangle as RectangleArea, Size,
    VerticalLine,
};
use crate::report::{BenchmarkId, ValueType};
use plotters::prelude::*;
use std::path::PathBuf;

mod distributions;
mod iteration_times;
mod pdf;
mod regression;
mod summary;
mod t_test;

static DEFAULT_FONT: FontFamily = FontFamily::SansSerif;
static SIZE: Size = Size(960, 540);
static POINT_SIZE: u32 = 3;

pub struct Colors {
    pub current_sample: RGBColor,
    pub previous_sample: RGBColor,
    pub not_an_outlier: RGBColor,
    pub mild_outlier: RGBColor,
    pub severe_outlier: RGBColor,
    pub comparison_colors: Vec<RGBColor>,
}
impl From<crate::config::Color> for RGBColor {
    fn from(other: crate::config::Color) -> Self {
        RGBColor(other.r, other.g, other.b)
    }
}
impl From<&crate::config::Colors> for Colors {
    fn from(other: &crate::config::Colors) -> Self {
        Colors {
            current_sample: other.current_sample.into(),
            previous_sample: other.previous_sample.into(),
            not_an_outlier: other.not_an_outlier.into(),
            mild_outlier: other.mild_outlier.into(),
            severe_outlier: other.severe_outlier.into(),
            comparison_colors: other
                .comparison_colors
                .iter()
                .copied()
                .map(RGBColor::from)
                .collect(),
        }
    }
}

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

pub struct PlottersBackend {
    colors: Colors,
}
impl PlottersBackend {
    pub fn new(colors: &crate::config::Colors) -> Self {
        PlottersBackend {
            colors: colors.into(),
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
        distributions::abs_distribution(
            &self.colors,
            id,
            statistic,
            size,
            path,
            x_unit,
            distribution_curve,
            bootstrap_area,
            point_estimate,
        )
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
        distributions::rel_distribution(
            &self.colors,
            id,
            statistic,
            size,
            path,
            distribution_curve,
            confidence_interval,
            point_estimate,
            noise_threshold,
        )
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
        iteration_times::iteration_times(
            &self.colors,
            id,
            size,
            path,
            unit,
            is_thumbnail,
            current_times,
            base_times,
        )
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
        regression::regression(
            &self.colors,
            id,
            size,
            path,
            is_thumbnail,
            x_label,
            x_scale,
            unit,
            sample,
            regression,
            confidence_interval,
        );
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
        regression::regression_comparison(
            &self.colors,
            id,
            size,
            path,
            is_thumbnail,
            x_label,
            x_scale,
            unit,
            current_regression,
            current_confidence_interval,
            base_regression,
            base_confidence_interval,
        );
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
        pdf::pdf_full(
            &self.colors,
            id,
            size,
            path,
            unit,
            y_label,
            y_scale,
            max_iters,
            pdf,
            mean,
            fences,
            points,
        );
    }

    fn pdf_thumbnail(
        &mut self,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        mean: Line,
        pdf: FilledCurve,
    ) {
        pdf::pdf_thumbnail(&self.colors, size, path, unit, mean, pdf);
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
        pdf::pdf_comparison(
            &self.colors,
            id,
            size,
            path,
            is_thumbnail,
            unit,
            current_mean,
            current_pdf,
            base_mean,
            base_pdf,
        );
    }

    fn t_test(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        t: VerticalLine,
        t_distribution: FilledCurve,
    ) {
        t_test::t_test(&self.colors, id, size, path, t, t_distribution);
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
        summary::line_comparison(
            &self.colors,
            path,
            title,
            unit,
            value_type,
            axis_scale,
            lines,
        );
    }

    fn violin(
        &mut self,
        path: PathBuf,
        title: &str,
        unit: &str,
        axis_scale: AxisScale,
        lines: &[(&str, LineCurve)],
    ) {
        summary::violin(&self.colors, path, title, unit, axis_scale, lines);
    }

    fn wait(&mut self) {}
}
