use crate::plot::gnuplot_backend::{gnuplot_escape, DARK_BLUE, DEFAULT_FONT, LINEWIDTH, SIZE};
use crate::plot::Size;
use crate::plot::{FilledCurve as FilledArea, VerticalLine};
use crate::report::BenchmarkId;
use criterion_plot::prelude::*;

pub fn t_test(
    id: &BenchmarkId,
    size: Option<Size>,
    t: VerticalLine,
    t_distribution: FilledArea,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .set(Title(format!(
            "{}: Welch t test",
            gnuplot_escape(id.as_title())
        )))
        .configure(Axis::BottomX, |a| a.set(Label("t score")))
        .configure(Axis::LeftY, |a| a.set(Label("Density")))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .plot(
            FilledCurve {
                x: t_distribution.xs,
                y1: t_distribution.ys_1,
                y2: t_distribution.ys_2,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(Label("t distribution"))
                    .set(Opacity(0.25))
            },
        )
        .plot(to_lines!(t, 1.0), |c| {
            c.set(Axes::BottomXRightY)
                .set(DARK_BLUE)
                .set(LINEWIDTH)
                .set(Label("t statistic"))
                .set(LineType::Solid)
        });

    figure
}
