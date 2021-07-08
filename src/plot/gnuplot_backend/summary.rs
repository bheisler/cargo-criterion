use crate::connection::AxisScale;
use crate::plot::gnuplot_backend::{
    gnuplot_escape, Colors, DEFAULT_FONT, LINEWIDTH, POINT_SIZE, SIZE,
};
use crate::plot::LineCurve;
use crate::plot::Size;
use crate::report::ValueType;
use criterion_plot::prelude::*;

pub fn line_comparison(
    colors: &Colors,
    title: &str,
    unit: &str,
    value_type: ValueType,
    axis_scale: AxisScale,
    lines: &[(Option<&String>, LineCurve)],
) -> Figure {
    let mut figure = Figure::new();

    let input_suffix = match value_type {
        ValueType::Bytes => " Size (Bytes)",
        ValueType::Elements => " Size (Elements)",
        ValueType::Value => "",
    };

    figure
        .set(Font(DEFAULT_FONT))
        .set(criterion_plot::Size::from(SIZE))
        .configure(Key, |k| {
            k.set(Justification::Left)
                .set(Order::SampleText)
                .set(Position::Outside(Vertical::Top, Horizontal::Right))
        })
        .set(Title(format!("{}: Comparison", gnuplot_escape(title))))
        .configure(Axis::BottomX, |a| {
            a.set(Label(format!("Input{}", input_suffix)))
                .set(axis_scale.to_gnuplot())
        });

    figure.configure(Axis::LeftY, |a| {
        a.configure(Grid::Major, |g| g.show())
            .configure(Grid::Minor, |g| g.hide())
            .set(Label(format!("Average time ({})", unit)))
            .set(axis_scale.to_gnuplot())
    });

    for (i, (name, curve)) in lines.iter().enumerate() {
        let function_name = name.map(|string| gnuplot_escape(string));

        figure
            .plot(
                Lines {
                    x: curve.xs,
                    y: curve.ys,
                },
                |c| {
                    if let Some(name) = function_name {
                        c.set(Label(name));
                    }
                    c.set(LINEWIDTH)
                        .set(LineType::Solid)
                        .set(colors.comparison_colors[i % colors.comparison_colors.len()])
                },
            )
            .plot(
                Points {
                    x: curve.xs,
                    y: curve.ys,
                },
                |p| {
                    p.set(PointType::FilledCircle)
                        .set(POINT_SIZE)
                        .set(colors.comparison_colors[i % colors.comparison_colors.len()])
                },
            );
    }

    figure
}

pub fn violin(
    colors: &Colors,
    title: &str,
    unit: &str,
    axis_scale: AxisScale,
    lines: &[(&str, LineCurve)],
) -> Figure {
    let tics = || (0..).map(|x| (f64::from(x)) + 0.5);
    let size: criterion_plot::Size = Size(1280, 200 + (25 * lines.len())).into();
    let mut figure = Figure::new();
    figure
        .set(Font(DEFAULT_FONT))
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
                .set(Range::Limits(0., lines.len() as f64))
                .set(TicLabels {
                    positions: tics(),
                    labels: lines.iter().map(|(id, _)| gnuplot_escape(id)),
                })
        });

    let mut is_first = true;
    for (i, (_, line)) in lines.iter().enumerate() {
        let i = i as f64 + 0.5;
        let y1: Vec<_> = line.ys.iter().map(|&y| i + y * 0.45).collect();
        let y2: Vec<_> = line.ys.iter().map(|&y| i - y * 0.45).collect();

        figure.plot(FilledCurve { x: line.xs, y1, y2 }, |c| {
            if is_first {
                is_first = false;

                c.set(colors.current_sample).set(Label("PDF"))
            } else {
                c.set(colors.current_sample)
            }
        });
    }
    figure
}
