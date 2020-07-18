use crate::connection::AxisScale;
use crate::estimate::Statistic;
use crate::format;
use crate::plot::Size;
use crate::plot::{
    FilledCurve as FilledArea, Line, LineCurve, PlottingBackend, Points as PointPlot, Rectangle,
    VerticalLine,
};
use crate::report::{BenchmarkId, ValueType};
use criterion_plot::prelude::*;
use std::path::{Path, PathBuf};
use std::process::Child;

macro_rules! to_lines {
    ($i:ident) => {
        Lines {
            x: &[$i.start.x, $i.end.x],
            y: &[$i.start.y, $i.end.y],
        }
    };
    ($i:ident, $max_y:expr) => {
        Lines {
            x: &[$i.x, $i.x],
            y: &[0.0, $max_y],
        }
    };
}

mod distributions;
mod history;
mod iteration_times;
mod pdf;
mod regression;
mod summary;
mod t_test;

fn gnuplot_escape(string: &str) -> String {
    string.replace("_", "\\_").replace("'", "''")
}

static DEFAULT_FONT: &str = "Helvetica";
static SIZE: Size = Size(1280, 720);

const LINEWIDTH: LineWidth = LineWidth(2.);
const POINT_SIZE: PointSize = PointSize(0.75);

pub struct Colors {
    pub current_sample: Color,
    pub previous_sample: Color,
    pub not_an_outlier: Color,
    pub mild_outlier: Color,
    pub severe_outlier: Color,
    pub comparison_colors: Vec<Color>,
}
impl From<crate::config::Color> for Color {
    fn from(other: crate::config::Color) -> Self {
        Color::Rgb(other.r, other.g, other.b)
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
                .map(Color::from)
                .collect(),
        }
    }
}

impl AxisScale {
    fn to_gnuplot(self) -> Scale {
        match self {
            AxisScale::Linear => Scale::Linear,
            AxisScale::Logarithmic => Scale::Logarithmic,
        }
    }
}

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

impl From<Size> for criterion_plot::Size {
    fn from(other: Size) -> Self {
        let Size(width, height) = other;
        Self(width, height)
    }
}

pub struct Gnuplot {
    process_list: Vec<Child>,
    colors: Colors,
}
impl Gnuplot {
    pub fn new(colors: &crate::config::Colors) -> Gnuplot {
        Gnuplot {
            process_list: vec![],
            colors: colors.into(),
        }
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
        let mut figure = distributions::abs_distribution(
            &self.colors,
            id,
            statistic,
            size,
            x_unit,
            distribution_curve,
            bootstrap_area,
            point_estimate,
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
        noise_threshold: Rectangle,
    ) {
        let mut figure = distributions::rel_distribution(
            &self.colors,
            id,
            statistic,
            size,
            distribution_curve,
            confidence_interval,
            point_estimate,
            noise_threshold,
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
        let mut figure = iteration_times::iteration_times(
            &self.colors,
            id,
            size,
            unit,
            is_thumbnail,
            current_times,
            base_times,
        );

        debug_script(&file_path, &figure);
        self.process_list
            .push(figure.set(Output(file_path)).draw().unwrap())
    }

    fn regression(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        file_path: PathBuf,
        is_thumbnail: bool,
        x_label: &str,
        x_scale: f64,
        unit: &str,
        sample: PointPlot,
        regression: Line,
        confidence_interval: FilledArea,
    ) {
        let mut figure = regression::regression(
            &self.colors,
            id,
            size,
            is_thumbnail,
            x_label,
            x_scale,
            unit,
            sample,
            regression,
            confidence_interval,
        );

        debug_script(&file_path, &figure);
        self.process_list
            .push(figure.set(Output(file_path)).draw().unwrap())
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
        current_confidence_interval: FilledArea,
        base_regression: Line,
        base_confidence_interval: FilledArea,
    ) {
        let mut figure = regression::regression_comparison(
            &self.colors,
            id,
            size,
            is_thumbnail,
            x_label,
            x_scale,
            unit,
            current_regression,
            current_confidence_interval,
            base_regression,
            base_confidence_interval,
        );
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
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
        pdf: FilledArea,
        mean: VerticalLine,
        fences: (VerticalLine, VerticalLine, VerticalLine, VerticalLine),
        points: (PointPlot, PointPlot, PointPlot),
    ) {
        let mut figure = pdf::pdf_full(
            &self.colors,
            id,
            size,
            unit,
            y_label,
            y_scale,
            max_iters,
            pdf,
            mean,
            fences,
            points,
        );

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn pdf_thumbnail(
        &mut self,
        size: Option<Size>,
        path: PathBuf,
        unit: &str,
        mean: Line,
        pdf: FilledArea,
    ) {
        let mut figure = pdf::pdf_thumbnail(&self.colors, size, unit, mean, pdf);
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn pdf_comparison(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        is_thumbnail: bool,
        unit: &str,
        current_mean: Line,
        current_pdf: FilledArea,
        base_mean: Line,
        base_pdf: FilledArea,
    ) {
        let mut figure = pdf::pdf_comparison(
            &self.colors,
            id,
            size,
            is_thumbnail,
            unit,
            current_mean,
            current_pdf,
            base_mean,
            base_pdf,
        );
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn t_test(
        &mut self,
        id: &BenchmarkId,
        size: Option<Size>,
        path: PathBuf,
        t: VerticalLine,
        t_distribution: FilledArea,
    ) {
        let mut figure = t_test::t_test(&self.colors, id, size, t, t_distribution);

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
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
        let mut figure =
            summary::line_comparison(&self.colors, title, unit, value_type, axis_scale, lines);

        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn violin(
        &mut self,
        path: PathBuf,
        title: &str,
        unit: &str,
        axis_scale: AxisScale,
        lines: &[(&str, LineCurve)],
    ) {
        let mut figure = summary::violin(&self.colors, title, unit, axis_scale, lines);
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
    }

    fn history_plot(
        &mut self,
        id: &BenchmarkId,
        size: Size,
        path: PathBuf,
        point_estimate: LineCurve,
        confidence_interval: FilledArea,
        ids: &[String],
        unit: &str,
    ) {
        let mut figure = history::history_plot(
            &self.colors,
            id.as_title(),
            size,
            point_estimate,
            confidence_interval,
            ids,
            unit,
        );
        debug_script(&path, &figure);
        self.process_list
            .push(figure.set(Output(path)).draw().unwrap())
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
