use crate::estimate::Statistic;
use crate::plot::gnuplot_backend::{
    gnuplot_escape, DARK_BLUE, DARK_RED, DEFAULT_FONT, LINEWIDTH, SIZE,
};
use crate::plot::Size;
use crate::plot::{FilledCurve as FilledArea, Line, LineCurve, Rectangle};
use crate::report::BenchmarkId;
use crate::stats::univariate::Sample;
use criterion_plot::prelude::*;

pub fn abs_distribution(
    id: &BenchmarkId,
    statistic: Statistic,
    size: Option<Size>,

    x_unit: &str,
    distribution_curve: LineCurve,
    bootstrap_area: FilledArea,
    point_estimate: Line,
) -> Figure {
    let xs_sample = Sample::new(distribution_curve.xs);

    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .set(Title(format!(
            "{}: {}",
            gnuplot_escape(id.as_title()),
            statistic
        )))
        .configure(Axis::BottomX, |a| {
            a.set(Label(format!("Average time ({})", x_unit)))
                .set(Range::Limits(xs_sample.min(), xs_sample.max()))
        })
        .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .plot(
            Lines {
                x: distribution_curve.xs,
                y: distribution_curve.ys,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("Bootstrap distribution"))
                    .set(LineType::Solid)
            },
        )
        .plot(
            FilledCurve {
                x: bootstrap_area.xs,
                y1: bootstrap_area.ys_1,
                y2: bootstrap_area.ys_2,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(Label("Confidence interval"))
                    .set(Opacity(0.25))
            },
        )
        .plot(
            Lines {
                x: &[point_estimate.start.x, point_estimate.end.x],
                y: &[point_estimate.start.y, point_estimate.end.y],
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("Point estimate"))
                    .set(LineType::Dash)
            },
        );
    figure
}

pub fn rel_distribution(
    id: &BenchmarkId,
    statistic: Statistic,
    size: Option<Size>,

    distribution_curve: LineCurve,
    confidence_interval: FilledArea,
    point_estimate: Line,
    noise_threshold: Rectangle,
) -> Figure {
    let xs_ = Sample::new(&distribution_curve.xs);
    let x_min = xs_.min();
    let x_max = xs_.max();

    let mut figure = Figure::new();

    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .set(Title(format!(
            "{}: {}",
            gnuplot_escape(id.as_title()),
            statistic
        )))
        .configure(Axis::BottomX, |a| {
            a.set(Label("Relative change (%)"))
                .set(Range::Limits(x_min * 100., x_max * 100.))
                .set(ScaleFactor(100.))
        })
        .plot(
            Lines {
                x: distribution_curve.xs,
                y: distribution_curve.ys,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(LINEWIDTH)
                    .set(Label("Bootstrap distribution"))
                    .set(LineType::Solid)
            },
        )
        .plot(
            FilledCurve {
                x: confidence_interval.xs,
                y1: confidence_interval.ys_1,
                y2: confidence_interval.ys_2,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(Label("Confidence interval"))
                    .set(Opacity(0.25))
            },
        )
        .plot(to_lines!(point_estimate), |c| {
            c.set(DARK_BLUE)
                .set(LINEWIDTH)
                .set(Label("Point estimate"))
                .set(LineType::Dash)
        })
        .plot(
            FilledCurve {
                x: &[noise_threshold.left, noise_threshold.right],
                y1: &[noise_threshold.bottom, noise_threshold.bottom],
                y2: &[noise_threshold.top, noise_threshold.top],
            },
            |c| {
                c.set(Axes::BottomXRightY)
                    .set(DARK_RED)
                    .set(Label("Noise threshold"))
                    .set(Opacity(0.1))
            },
        );
    figure
}
