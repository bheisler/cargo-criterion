use super::*;
use crate::connection::AxisScale;
use crate::model::Benchmark;
use crate::plot::KDE_POINTS;
use plotters::coord::{AsRangedCoord, Shift};
use std::path::Path;

pub fn violin(
    formatter: &dyn ValueFormatter,
    title: &str,
    all_benchmarks: &[(&BenchmarkId, &Benchmark)],
    path: &Path,
    axis_scale: AxisScale,
) {
    let mut kdes = all_benchmarks
        .iter()
        .rev()
        .map(|(id, sample)| {
            let (x, mut y) = kde::sweep(
                Sample::new(&sample.latest_stats.avg_values),
                KDE_POINTS,
                None,
            );
            let y_max = Sample::new(&y).max();
            for y in y.iter_mut() {
                *y /= y_max;
            }

            (id.as_title(), x, y)
        })
        .collect::<Vec<_>>();

    let mut xs = kdes
        .iter()
        .flat_map(|&(_, ref x, _)| x.iter())
        .filter(|&&x| x > 0.);
    let (mut min, mut max) = {
        let &first = xs.next().unwrap();
        (first, first)
    };
    for &e in xs {
        if e < min {
            min = e;
        } else if e > max {
            max = e;
        }
    }
    let mut dummy = [1.0];
    let unit = formatter.scale_values(max, &mut dummy);
    kdes.iter_mut().for_each(|&mut (_, ref mut xs, _)| {
        formatter.scale_values(max, xs);
    });

    let x_range = plotters::data::fitting_range(kdes.iter().map(|(_, xs, _)| xs.iter()).flatten());
    let y_range = -0.5..all_benchmarks.len() as f64 - 0.5;

    let size = (960, 150 + (18 * all_benchmarks.len() as u32));

    let root_area = SVGBackend::new(&path, size)
        .into_drawing_area()
        .titled(&format!("{}: Violin plot", title), (DEFAULT_FONT, 20))
        .unwrap();

    match axis_scale {
        AxisScale::Linear => draw_violin_figure(root_area, &unit, x_range, y_range, kdes),
        AxisScale::Logarithmic => {
            draw_violin_figure(root_area, &unit, LogRange(x_range), y_range, kdes)
        }
    }
}

#[allow(clippy::type_complexity)]
fn draw_violin_figure<XR: AsRangedCoord<Value = f64>, YR: AsRangedCoord<Value = f64>>(
    root_area: DrawingArea<SVGBackend, Shift>,
    unit: &str,
    x_range: XR,
    y_range: YR,
    data: Vec<(&str, Box<[f64]>, Box<[f64]>)>,
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

    for (i, (_, x, y)) in data.into_iter().enumerate() {
        let base = i as f64;

        chart
            .draw_series(AreaSeries::new(
                x.iter().zip(y.iter()).map(|(x, y)| (*x, base + *y / 2.0)),
                base,
                &DARK_BLUE.mix(0.25),
            ))
            .unwrap();

        chart
            .draw_series(AreaSeries::new(
                x.iter().zip(y.iter()).map(|(x, y)| (*x, base - *y / 2.0)),
                base,
                &DARK_BLUE.mix(0.25),
            ))
            .unwrap();
    }
}
