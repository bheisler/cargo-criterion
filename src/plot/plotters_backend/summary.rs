use crate::connection::AxisScale;
use crate::plot::plotters_backend::{
    COMPARISON_COLORS, DARK_BLUE, DEFAULT_FONT, NUM_COLORS, POINT_SIZE, SIZE,
};
use crate::plot::LineCurve;
use crate::report::ValueType;
use plotters::coord::{AsRangedCoord, Shift};
use plotters::prelude::*;
use std::path::PathBuf;

pub fn line_comparison(
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
        AxisScale::Linear => {
            draw_line_comparison_figure(root_area, &unit, x_range, y_range, value_type, lines)
        }
        AxisScale::Logarithmic => draw_line_comparison_figure(
            root_area,
            &unit,
            LogRange(x_range),
            LogRange(y_range),
            value_type,
            lines,
        ),
    }
}

fn draw_line_comparison_figure<XR: AsRangedCoord<Value = f64>, YR: AsRangedCoord<Value = f64>>(
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

    for (id, (name, curve)) in data.iter().enumerate() {
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

pub fn violin(
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
        AxisScale::Linear => draw_violin_figure(root_area, &unit, x_range, y_range, lines),
        AxisScale::Logarithmic => {
            draw_violin_figure(root_area, &unit, LogRange(x_range), y_range, lines)
        }
    }
}

fn draw_violin_figure<XR: AsRangedCoord<Value = f64>, YR: AsRangedCoord<Value = f64>>(
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

    for (i, (_, curve)) in data.iter().enumerate() {
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
