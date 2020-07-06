use crate::plot::gnuplot_backend::{gnuplot_escape, Colors, DEFAULT_FONT, LINEWIDTH, SIZE};
use crate::plot::Points as PointPlot;
use crate::plot::Size;
use crate::plot::{FilledCurve as FilledArea, Line};
use crate::report::BenchmarkId;
use criterion_plot::prelude::*;

pub fn regression(
    colors: &Colors,
    id: &BenchmarkId,
    size: Option<Size>,
    is_thumbnail: bool,
    x_label: &str,
    x_scale: f64,
    unit: &str,
    sample: PointPlot,
    regression: Line,
    confidence_interval: FilledArea,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(x_label.to_owned()))
                .set(ScaleFactor(x_scale))
        })
        .configure(Axis::LeftY, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(format!("Total sample time ({})", unit)))
        })
        .plot(
            Points {
                x: sample.xs,
                y: sample.ys,
            },
            |c| {
                c.set(colors.current_sample)
                    .set(Label("Sample"))
                    .set(PointSize(0.5))
                    .set(PointType::FilledCircle)
            },
        )
        .plot(to_lines!(regression), |c| {
            c.set(colors.current_sample)
                .set(LINEWIDTH)
                .set(Label("Linear regression"))
                .set(LineType::Solid)
        })
        .plot(
            FilledCurve {
                x: confidence_interval.xs,
                y1: confidence_interval.ys_1,
                y2: confidence_interval.ys_2,
            },
            |c| {
                c.set(colors.current_sample)
                    .set(Label("Confidence interval"))
                    .set(Opacity(0.25))
            },
        );

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

pub fn regression_comparison(
    colors: &Colors,
    id: &BenchmarkId,
    size: Option<Size>,
    is_thumbnail: bool,
    x_label: &str,
    x_scale: f64,
    unit: &str,
    current_regression: Line,
    current_confidence_interval: FilledArea,
    base_regression: Line,
    base_confidence_interval: FilledArea,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(x_label.to_owned()))
                .set(ScaleFactor(x_scale))
        })
        .configure(Axis::LeftY, |a| {
            a.configure(Grid::Major, |g| g.show())
                .set(Label(format!("Total sample time ({})", unit)))
        })
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Inside(Vertical::Top, Horizontal::Left))
        })
        .plot(
            FilledCurve {
                x: base_confidence_interval.xs,
                y1: base_confidence_interval.ys_1,
                y2: base_confidence_interval.ys_2,
            },
            |c| c.set(colors.previous_sample).set(Opacity(0.25)),
        )
        .plot(
            FilledCurve {
                x: current_confidence_interval.xs,
                y1: current_confidence_interval.ys_1,
                y2: current_confidence_interval.ys_2,
            },
            |c| c.set(colors.current_sample).set(Opacity(0.25)),
        )
        .plot(to_lines!(base_regression), |c| {
            c.set(colors.previous_sample)
                .set(LINEWIDTH)
                .set(Label("Base sample"))
                .set(LineType::Solid)
        })
        .plot(to_lines!(current_regression), |c| {
            c.set(colors.current_sample)
                .set(LINEWIDTH)
                .set(Label("New sample"))
                .set(LineType::Solid)
        });

    if is_thumbnail {
        figure.configure(Key, |k| k.hide());
    } else {
        figure.set(Title(gnuplot_escape(id.as_title())));
    }

    figure
}
