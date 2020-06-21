#[cfg(feature = "gnuplot_backend")]
mod gnuplot_backend;
#[cfg(feature = "plotters_backend")]
mod plotters_backend;

#[cfg(feature = "gnuplot_backend")]
pub(crate) use gnuplot_backend::Gnuplot;
#[cfg(feature = "plotters_backend")]
pub(crate) use plotters_backend::PlottersBackend;

use crate::model::Benchmark;
use crate::report::{BenchmarkId, ComparisonData, MeasurementData, ReportContext, ValueType};
use crate::value_formatter::ValueFormatter;
use std::path::PathBuf;

#[derive(Clone, Copy)]
pub(crate) struct PlotContext<'a> {
    pub(crate) id: &'a BenchmarkId,
    pub(crate) context: &'a ReportContext,
    pub(crate) size: Option<(usize, usize)>,
    pub(crate) is_thumbnail: bool,
}

impl<'a> PlotContext<'a> {
    pub fn size(mut self, s: Option<crate::html::Size>) -> PlotContext<'a> {
        if let Some(s) = s {
            self.size = Some((s.0, s.1));
        }
        self
    }

    pub fn thumbnail(mut self, value: bool) -> PlotContext<'a> {
        self.is_thumbnail = value;
        self
    }

    pub fn line_comparison_path(&self) -> PathBuf {
        path!(
            &self.context.output_directory,
            self.id.as_directory_name(),
            "report",
            "lines.svg"
        )
    }

    pub fn violin_path(&self) -> PathBuf {
        path!(
            &self.context.output_directory,
            self.id.as_directory_name(),
            "report",
            "violin.svg"
        )
    }
}

#[derive(Clone, Copy)]
pub(crate) struct PlotData<'a> {
    pub(crate) formatter: &'a dyn ValueFormatter,
    pub(crate) measurements: &'a MeasurementData<'a>,
    pub(crate) comparison: Option<&'a ComparisonData>,
}

impl<'a> PlotData<'a> {
    pub fn comparison(mut self, comp: &'a ComparisonData) -> PlotData<'a> {
        self.comparison = Some(comp);
        self
    }
}

pub(crate) trait Plotter {
    fn pdf(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn regression(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn abs_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn rel_distributions(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn line_comparison(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
        value_type: ValueType,
    );

    fn violin(
        &mut self,
        ctx: PlotContext<'_>,
        formatter: &dyn ValueFormatter,
        all_curves: &[(&BenchmarkId, &Benchmark)],
    );

    fn t_test(&mut self, ctx: PlotContext<'_>, data: PlotData<'_>);

    fn wait(&mut self);
}
