use crate::plot::gnuplot_backend::{
    gnuplot_escape, DARK_BLUE, DARK_ORANGE, DARK_RED, DEFAULT_FONT, LINEWIDTH, POINT_SIZE, SIZE,
};
use crate::plot::Size;
use crate::plot::{FilledCurve as FilledArea, Line, Points as PointPlot, VerticalLine};
use crate::report::BenchmarkId;
use crate::stats::univariate::Sample;
use criterion_plot::prelude::*;

pub fn pdf_full(
    id: &BenchmarkId,
    size: Option<Size>,
    unit: &str,
    y_label: &str,
    y_scale: f64,
    max_iters: f64,
    pdf: FilledArea,
    mean: VerticalLine,
    fences: (VerticalLine, VerticalLine, VerticalLine, VerticalLine),
    points: (PointPlot, PointPlot, PointPlot),
) -> Figure {
    let (low_severe, low_mild, high_mild, high_severe) = fences;
    let (not_outlier, mild, severe) = points;

    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            let xs_ = Sample::new(&pdf.xs);
            a.set(Label(format!("Average time ({})", unit)))
                .set(Range::Limits(xs_.min(), xs_.max()))
        })
        .configure(Axis::LeftY, |a| {
            a.set(Label(y_label.to_owned()))
                .set(Range::Limits(0., max_iters * y_scale))
                .set(ScaleFactor(y_scale))
        })
        .configure(Axis::RightY, |a| a.set(Label("Density (a.u.)")))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .plot(
            FilledCurve {
                x: pdf.xs,
                y1: pdf.ys_1,
                y2: pdf.ys_2,
            },
            |c| {
                c.set(Axes::BottomXRightY)
                    .set(DARK_BLUE)
                    .set(Label("PDF"))
                    .set(Opacity(0.25))
            },
        )
        .plot(to_lines!(mean, max_iters), |c| {
            c.set(DARK_BLUE)
                .set(LINEWIDTH)
                .set(LineType::Dash)
                .set(Label("Mean"))
        })
        .plot(
            Points {
                x: not_outlier.xs,
                y: not_outlier.ys,
            },
            |c| {
                c.set(DARK_BLUE)
                    .set(Label("\"Clean\" sample"))
                    .set(PointType::FilledCircle)
                    .set(POINT_SIZE)
            },
        )
        .plot(
            Points {
                x: mild.xs,
                y: mild.ys,
            },
            |c| {
                c.set(DARK_ORANGE)
                    .set(Label("Mild outliers"))
                    .set(POINT_SIZE)
                    .set(PointType::FilledCircle)
            },
        )
        .plot(
            Points {
                x: severe.xs,
                y: severe.ys,
            },
            |c| {
                c.set(DARK_RED)
                    .set(Label("Severe outliers"))
                    .set(POINT_SIZE)
                    .set(PointType::FilledCircle)
            },
        )
        .plot(to_lines!(low_mild, max_iters), |c| {
            c.set(DARK_ORANGE).set(LINEWIDTH).set(LineType::Dash)
        })
        .plot(to_lines!(high_mild, max_iters), |c| {
            c.set(DARK_ORANGE).set(LINEWIDTH).set(LineType::Dash)
        })
        .plot(to_lines!(low_severe, max_iters), |c| {
            c.set(DARK_RED).set(LINEWIDTH).set(LineType::Dash)
        })
        .plot(to_lines!(high_severe, max_iters), |c| {
            c.set(DARK_RED).set(LINEWIDTH).set(LineType::Dash)
        });
    figure.set(Title(gnuplot_escape(id.as_title())));
    figure
}

pub fn pdf_thumbnail(size: Option<Size>, unit: &str, mean: Line, pdf: FilledArea) -> Figure {
    let xs_ = Sample::new(pdf.xs);
    let ys_ = Sample::new(pdf.ys_1);
    let y_limit = ys_.max() * 1.1;

    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            a.set(Label(format!("Average time ({})", unit)))
                .set(Range::Limits(xs_.min(), xs_.max()))
        })
        .configure(Axis::LeftY, |a| {
            a.set(Label("Density (a.u.)"))
                .set(Range::Limits(0., y_limit))
        })
        .configure(Axis::RightY, |a| a.hide())
        .configure(Key, |k| k.hide())
        .plot(
            FilledCurve {
                x: pdf.xs,
                y1: pdf.ys_1,
                y2: pdf.ys_2,
            },
            |c| {
                c.set(Axes::BottomXRightY)
                    .set(DARK_BLUE)
                    .set(Label("PDF"))
                    .set(Opacity(0.25))
            },
        )
        .plot(to_lines!(mean), |c| {
            c.set(DARK_BLUE).set(LINEWIDTH).set(Label("Mean"))
        });

    figure
}

pub fn pdf_comparison(
    id: &BenchmarkId,
    size: Option<Size>,
    is_thumbnail: bool,
    unit: &str,
    current_mean: Line,
    current_pdf: FilledArea,
    base_mean: Line,
    base_pdf: FilledArea,
) -> Figure {
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(size.unwrap_or(SIZE)))
        .configure(Axis::BottomX, |a| {
            a.set(Label(format!("Average time ({})", unit)))
        })
        .configure(Axis::LeftY, |a| a.set(Label("Density (a.u.)")))
        .configure(Axis::RightY, |a| a.hide())
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .plot(
            FilledCurve {
                x: base_pdf.xs,
                y1: base_pdf.ys_1,
                y2: base_pdf.ys_2,
            },
            |c| c.set(DARK_RED).set(Label("Base PDF")).set(Opacity(0.5)),
        )
        .plot(to_lines!(base_mean), |c| {
            c.set(DARK_RED).set(Label("Base Mean")).set(LINEWIDTH)
        })
        .plot(
            FilledCurve {
                x: current_pdf.xs,
                y1: current_pdf.ys_1,
                y2: current_pdf.ys_2,
            },
            |c| c.set(DARK_BLUE).set(Label("New PDF")).set(Opacity(0.5)),
        )
        .plot(to_lines!(current_mean), |c| {
            c.set(DARK_BLUE).set(Label("New Mean")).set(LINEWIDTH)
        });

    if is_thumbnail {
        figure.configure(Key, |k| k.hide());
    } else {
        figure.set(Title(gnuplot_escape(id.as_title())));
    }
    figure
}
