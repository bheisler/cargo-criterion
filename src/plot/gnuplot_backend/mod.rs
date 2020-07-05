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

const DARK_BLUE: Color = Color::Rgb(31, 120, 180);
const DARK_ORANGE: Color = Color::Rgb(255, 127, 0);
const DARK_RED: Color = Color::Rgb(227, 26, 28);

const NUM_COLORS: usize = 8;
static COMPARISON_COLORS: [Color; NUM_COLORS] = [
    Color::Rgb(178, 34, 34),
    Color::Rgb(46, 139, 87),
    Color::Rgb(0, 139, 139),
    Color::Rgb(255, 215, 0),
    Color::Rgb(0, 0, 139),
    Color::Rgb(220, 20, 60),
    Color::Rgb(139, 0, 139),
    Color::Rgb(0, 255, 127),
];

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
            id, size, unit, y_label, y_scale, max_iters, pdf, mean, fences, points,
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
        let mut figure = pdf::pdf_thumbnail(size, unit, mean, pdf);
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
        let mut figure = t_test::t_test(id, size, t, t_distribution);

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
        let mut figure = summary::line_comparison(title, unit, value_type, axis_scale, lines);

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
        let mut figure = summary::violin(title, unit, axis_scale, lines);
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
