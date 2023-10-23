use num::{complex::ComplexFloat, traits::FloatConst, Float};

use super::{
    band_filter::GenericBandFilter,
    iir_filter,
    output_type::{DesiredFilterOutput, GenericFilterOutput, GenericZpk},
    GenericAnalog,
};

pub mod bessel;
pub mod butter;
pub mod butterord;
pub mod cheby1;
pub mod cheby2;
pub mod ellip;

pub struct GenericFilterSettings<T> {
    pub order: u32,
    pub band_filter: GenericBandFilter<T>,
    pub analog: GenericAnalog<T>,
}

impl<T> GenericFilterSettings<T> {}

pub trait ProtoFilter<T: Float + FloatConst + ComplexFloat> {
    fn proto_filter(&self) -> GenericZpk<T>;

    fn filter_settings(&self) -> &GenericFilterSettings<T>;
}

pub trait FilterDesign<T: Float + FloatConst + ComplexFloat>: ProtoFilter<T> {
    fn filter(&self, desired_output: DesiredFilterOutput) -> GenericFilterOutput<T> {
        let proto = self.proto_filter();
        let settings = self.filter_settings();
        iir_filter(
            proto,
            settings.order.clone(),
            settings.band_filter.clone(),
            settings.analog.clone(),
            desired_output,
        )
    }
}

impl<T: Float + FloatConst + ComplexFloat, K> FilterDesign<T> for K where K: ProtoFilter<T> {}
