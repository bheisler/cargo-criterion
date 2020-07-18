use crate::plot::gnuplot_backend::{gnuplot_escape, Colors, DEFAULT_FONT, LINEWIDTH};
use crate::plot::Size;
use crate::plot::{FilledCurve as FilledArea, LineCurve};
use criterion_plot::prelude::*;

pub fn history_plot(
    colors: &Colors,
    title: &str,
    size: Size,
    point_estimate: LineCurve,
    confidence_interval: FilledArea,
    ids: &[String],
    unit: &str,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .set(Title(format!("{}: History", gnuplot_escape(title))))
        .configure(Axis::BottomX, |a| {
            a.set(Label("Benchmark")).set(TicLabels {
                labels: ids,
                positions: point_estimate.xs,
            })
        })
        .configure(Axis::LeftY, |a| {
            a.set(Label(format!("Average time ({})", unit)))
        });

    figure.plot(
        Lines {
            x: point_estimate.xs,
            y: point_estimate.ys,
        },
        |c| {
            c.set(colors.current_sample)
                .set(LINEWIDTH)
                .set(Label("Point estimate"))
        },
    );
    figure.plot(
        FilledCurve {
            x: confidence_interval.xs,
            y1: confidence_interval.ys_1,
            y2: confidence_interval.ys_2,
        },
        |c| {
            c.set(colors.current_sample)
                .set(Opacity(0.5))
                .set(Label("Confidence Interval"))
        },
    );
    figure
}
