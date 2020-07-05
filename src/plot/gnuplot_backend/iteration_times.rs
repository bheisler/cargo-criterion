use crate::plot::gnuplot_backend::{gnuplot_escape, DARK_BLUE, DARK_RED, DEFAULT_FONT, SIZE};
use crate::plot::Points as PointPlot;
use crate::plot::Size;
use crate::report::BenchmarkId;
use criterion_plot::prelude::*;

pub fn iteration_times(
    id: &BenchmarkId,
    size: Option<Size>,

    unit: &str,
    is_thumbnail: bool,
    current_times: PointPlot,
    base_times: Option<PointPlot>,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            a.configure(Grid::Major, |g| g.show()).set(Label("Sample"))
        })
        .configure(Axis::LeftY, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(format!("Average Iteration Time ({})", unit)))
        })
        .plot(
            Points {
                x: current_times.xs,
                y: current_times.ys,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(Label("Current"))
                    .set(PointSize(0.5))
                    .set(PointType::FilledCircle)
            },
        );

    if let Some(base_times) = base_times {
        figure.plot(
            Points {
                x: base_times.xs,
                y: base_times.ys,
            },
            |c| {
                c.set(DARK_RED)
                    .set(Label("Base"))
                    .set(PointSize(0.5))
                    .set(PointType::FilledCircle)
            },
        );
    }

    if !is_thumbnail {
        figure.set(Title(gnuplot_escape(id.as_title())));
        figure.configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Inside(Vertical::Top, Horizontal::Left))
        });
    } else {
        figure.configure(Key, |k| k.hide());
    }

    figure
}
