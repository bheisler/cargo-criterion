use super::{debug_script, gnuplot_escape, DARK_BLUE, DEFAULT_FONT};
use crate::connection::AxisScale;
use crate::kde;
use crate::model::Benchmark;
use crate::plot::KDE_POINTS;
use crate::report::BenchmarkId;
use crate::stats::univariate::Sample;
use crate::value_formatter::ValueFormatter;
use criterion_plot::prelude::*;
use std::path::PathBuf;
use std::process::Child;

pub fn violin(
    formatter: &dyn ValueFormatter,
    title: &str,
    all_benchmarks: &[(&BenchmarkId, &Benchmark)],
    path: PathBuf,
    axis_scale: AxisScale,
) -> Child {
    let kdes = all_benchmarks
        .iter()
        .rev()
        .map(|(_, benchmark)| {
            let (x, mut y) = kde::sweep(
                Sample::new(&benchmark.latest_stats.avg_values),
                KDE_POINTS,
                None,
            );
            let y_max = Sample::new(&y).max();
            for y in y.iter_mut() {
                *y /= y_max;
            }

            (x, y)
        })
        .collect::<Vec<_>>();
    let mut xs = kdes
        .iter()
        .flat_map(|&(ref x, _)| x.iter())
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
    let mut one = [1.0];
    // Scale the X axis units. Use the middle as a "typical value". E.g. if
    // it is 0.002 s then this function will decide that milliseconds are an
    // appropriate unit. It will multiple `one` by 1000, and return "ms".
    let unit = formatter.scale_values((min + max) / 2.0, &mut one);

    let tics = || (0..).map(|x| (f64::from(x)) + 0.5);
    let size = Size(1280, 200 + (25 * all_benchmarks.len()));
    let mut f = Figure::new();
    f.set(Font(DEFAULT_FONT))
        .set(size)
        .set(Title(format!("{}: Violin plot", gnuplot_escape(title))))
        .configure(Axis::BottomX, |a| {
            a.configure(Grid::Major, |g| g.show())
                .configure(Grid::Minor, |g| g.hide())
                .set(Label(format!("Average time ({})", unit)))
                .set(axis_scale.to_gnuplot())
        })
        .configure(Axis::LeftY, |a| {
            a.set(Label("Input"))
                .set(Range::Limits(0., all_benchmarks.len() as f64))
                .set(TicLabels {
                    positions: tics(),
                    labels: all_benchmarks
                        .iter()
                        .rev()
                        .map(|(id, _)| gnuplot_escape(id.as_title())),
                })
        });

    let mut is_first = true;
    for (i, &(ref x, ref y)) in kdes.iter().enumerate() {
        let i = i as f64 + 0.5;
        let y1: Vec<_> = y.iter().map(|&y| i + y * 0.45).collect();
        let y2: Vec<_> = y.iter().map(|&y| i - y * 0.45).collect();

        let x: Vec<_> = x.iter().map(|&x| x * one[0]).collect();

        f.plot(FilledCurve { x, y1, y2 }, |c| {
            if is_first {
                is_first = false;

                c.set(DARK_BLUE).set(Label("PDF")).set(Opacity(0.25))
            } else {
                c.set(DARK_BLUE).set(Opacity(0.25))
            }
        });
    }
    debug_script(&path, &f);
    f.set(Output(path)).draw().unwrap()
}
