

use ndarray::{array, Array1};
use num::{complex::ComplexFloat, traits::FloatConst, Complex, Float, NumCast, One, Zero};

use crate::{
    optimize::{Metric},
    signal::{
        output_type::GenericZpk,
        tools::{newton, polyval},
    },
    special::kve,
    tools::complex::normalize_zeros,
};

use super::{GenericFilterSettings, ProtoFilter};

pub struct BesselFilter<T> {
    pub norm: BesselNorm,
    pub settings: GenericFilterSettings<T>,
}

impl<T: Float + FloatConst + ComplexFloat + Clone + Metric + 'static> ProtoFilter<T>
    for BesselFilter<T>
{
    fn proto_filter(&self) -> crate::signal::output_type::GenericZpk<T> {
        besselap(self.settings.order, self.norm)
    }

    fn filter_settings(&self) -> &GenericFilterSettings<T> {
        &self.settings
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BesselNorm {
    Phase,
    Delay,
    Mag,
}

// TODO! _norm defaults to Phase, other normalizations are not implemented
pub fn besselap<T: Float + FloatConst + Metric + 'static>(
    order: u32,
    norm: BesselNorm,
) -> GenericZpk<T> {
    let z = array![];
    let mut p: Array1<Complex<T>>;
    let mut k = T::one();
    if order == 0 {
        p = array![];
    } else {
        let a_last: T = (_falling_factorial::<T>(2 * order, order)
            / T::from(2.0).unwrap().powi(order as i32))
        .floor();
        p = _bessel_zeros::<T>(order)
            .into_iter()
            .map(|a| Complex::new(T::from(1.0).unwrap(), T::zero()) / a)
            .collect();

        if norm == BesselNorm::Delay || norm == BesselNorm::Mag {
            k = a_last;
            if norm == BesselNorm::Mag {
                let norm_factor = _norm_factor(p.clone(), k);
                p = p.mapv(|a| a / norm_factor);
                k = norm_factor.powf(-T::from(order).unwrap()) * a_last;
            }
        } else {
            p.iter_mut().for_each(|a| {
                *a = *a
                    * T::from(10.0)
                        .unwrap()
                        .powf(-a_last.log10() / T::from(order).unwrap())
            });
        }
    }
    let p = normalize_zeros(p);
    GenericZpk { z, p, k }
}

fn _norm_factor<T: Float + FloatConst + Metric + 'static>(p: Array1<Complex<T>>, k: T) -> T {
    let g = move |w: T| {
        let tmp = p.mapv(|a| Complex::i() * w - a);
        let tmp = Complex::new(k, T::zero()) / tmp.product();
        tmp.abs()
    };
    let cutoff = move |w: T| g(w) - T::one() / T::from(2).unwrap().sqrt();

    crate::optimize::root_scalar::secant_method(
        cutoff,
        T::from(1.5).unwrap(),
        T::from(1.5 * (1.0 + 1.0e-4)).unwrap(),
        None,
    );
    todo!()
}

fn _falling_factorial<T: Float>(x: u32, n: u32) -> T {
    let mut y = 1.0;

    for i in (x - n + 1)..(x + 1) {
        y *= i as f64;
    }
    T::from(y).unwrap()
}

fn _bessel_zeros<T: Float + FloatConst>(order: u32) -> Array1<Complex<T>> {
    if order == 0 {
        return array![];
    }

    let x0 = _campos_zeros(order);
    let f = |x: Complex<T>| {
        let x = Complex::new(
            <f64 as NumCast>::from(x.re).unwrap(),
            <f64 as NumCast>::from(x.im).unwrap(),
        );
        let r = kve(order as f64 + 0.5, 1.0 / x);
        Complex::new(T::from(r.re).unwrap(), T::from(r.im).unwrap())
    };

    let fp = |x: Complex<T>| {
        let x = Complex::new(
            <f64 as num::NumCast>::from(x.re).unwrap(),
            <f64 as num::NumCast>::from(x.im).unwrap(),
        );
        let order = order as f64;

        let first = kve(order - 0.5, 1.0 / x) / (2.0 * x.powi(2));
        let second = kve(order + 0.5, 1.0 / x) / x.powi(2);
        let third = kve(order + 1.5, 1.0 / x) / (2.0 * x.powi(2));
        let r = first - second + third;
        Complex::new(T::from(r.re).unwrap(), T::from(r.im).unwrap())
    };
    let mut x = _aberth(f, fp, &x0);

    for i in &mut x {
        *i = newton(f, fp, *i, T::from(10.0_f64.powi(-16)).unwrap(), 50);
    }

    let clone = x.clone().into_iter().map(|a| a.conj()).rev();

    let temp = x.iter().copied().zip(clone);
    let x: Array1<Complex<T>> = temp.map(|(a, b)| (a + b) / T::from(2.0).unwrap()).collect();

    x
}

fn _aberth<
    T: Float + FloatConst,
    F: Fn(Complex<T>) -> Complex<T>,
    FP: Fn(Complex<T>) -> Complex<T>,
>(
    f: F,
    fp: FP,
    x0: &[Complex<T>],
) -> Vec<Complex<T>> {
    let mut zs = x0.to_vec();
    let mut new_zs = zs.clone();
    let tol = T::from(10.0_f64.powi(-16)).unwrap();
    'iteration: for _ in 0..100 {
        for i in 0..(x0.len()) {
            let p_of_z = f(zs[i]);
            let dydx_of_z = fp(zs[i]);

            let sum: Complex<T> = (0..zs.len())
                .filter(|&k| k != i)
                .fold(Complex::zero(), |acc: Complex<T>, k| {
                    acc + Complex::<T>::one() / (zs[i] - zs[k])
                });

            let new_z = zs[i] + p_of_z / (p_of_z * sum - dydx_of_z);
            new_zs[i] = new_z;
            if new_z.re.is_nan()
                || new_z.im.is_nan()
                || new_z.re.is_infinite()
                || new_z.im.is_infinite()
            {
                break 'iteration;
            }
            let err = (new_z - zs[i]).abs();
            if err < tol {
                return new_zs;
            }

            zs = new_zs.clone();
        }
    }

    panic!();
}

// verified with python
fn _campos_zeros<T: Float>(order: u32) -> Vec<Complex<T>> {
    let n = order as _;
    if n == 1.0 {
        return vec![Complex::new(-T::one(), T::zero())];
    }
    let s = polyval(n, [0.0, 0.0, 2.0, 0.0, -3.0, 1.0]);
    let b3 = polyval(n, [16.0, -8.0]) / s;
    let b2 = polyval(n, [-24.0, -12.0, 12.0]) / s;
    let b1 = polyval(n, [8.0, 24.0, -12.0, -2.0]) / s;
    let b0 = polyval(n, [0.0, -6.0, 0.0, 5.0, -1.0]) / s;

    let r = polyval(n, [0.0, 0.0, 2.0, 1.0]);

    let a1 = polyval(n, [-6.0, -6.0]) / r;
    let a2 = 6.0 / r;

    let k = 1..(order + 1);

    let x = k
        .clone()
        .map(|a| polyval(Complex::new(a as f64, 0.0), [0.0.into(), a1, a2]))
        .collect::<Vec<_>>();
    let y = k
        .map(|a| polyval(Complex::new(a as f64, 0.0), [b0, b1, b2, b3]))
        .collect::<Vec<_>>();

    assert_eq!(x.len(), y.len());
    x.iter()
        .zip(y)
        .map(|(x, y)| *x + Complex::new(0.0, 1.0) * y)
        .map(|a| Complex::new(T::from(a.re).unwrap(), T::from(a.im).unwrap()))
        .collect::<Vec<_>>()
}
