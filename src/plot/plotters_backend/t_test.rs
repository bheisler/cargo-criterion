use crate::plot::plotters_backend::{DARK_BLUE, DEFAULT_FONT, SIZE};
use crate::plot::{FilledCurve, Size, VerticalLine};
use crate::report::BenchmarkId;
use plotters::prelude::*;
use std::path::PathBuf;

pub fn t_test(
    id: &BenchmarkId,
    size: Option<Size>,
    path: PathBuf,
    t: VerticalLine,
    t_distribution: FilledCurve,
) {
    let x_range = plotters::data::fitting_range(t_distribution.xs.iter());
    let mut y_range = plotters::data::fitting_range(t_distribution.ys_1.iter());
    y_range.start = 0.0;
    y_range.end *= 1.1;

    let root_area = SVGBackend::new(&path, size.unwrap_or(SIZE).into()).into_drawing_area();

    let mut chart = ChartBuilder::on(&root_area)
        .margin((5).percent())
        .caption(
            format!("{}: Welch t test", id.as_title()),
            (DEFAULT_FONT, 20),
        )
        .set_label_area_size(LabelAreaPosition::Left, (5).percent_width().min(60))
        .set_label_area_size(LabelAreaPosition::Bottom, (5).percent_height().min(40))
        .build_ranged(x_range, y_range.clone())
        .unwrap();

    chart
        .configure_mesh()
        .disable_mesh()
        .y_desc("Density")
        .x_desc("t score")
        .draw()
        .unwrap();

    chart
        .draw_series(AreaSeries::new(
            t_distribution.to_points(),
            0.0,
            &DARK_BLUE.mix(0.25),
        ))
        .unwrap()
        .label("t distribution")
        .legend(|(x, y)| {
            Rectangle::new([(x, y - 5), (x + 20, y + 5)], DARK_BLUE.mix(0.25).filled())
        });

    chart
        .draw_series(std::iter::once(PathElement::new(
            t.to_line_vec(y_range.end),
            DARK_BLUE.filled().stroke_width(2),
        )))
        .unwrap()
        .label("t statistic")
        .legend(|(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], &DARK_BLUE));

    chart.configure_series_labels().draw().unwrap();
}
