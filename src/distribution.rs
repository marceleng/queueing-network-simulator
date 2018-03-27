extern crate rand;

use distribution::rand::Rng;

use distribution::rand::distributions::{Exp, Sample};

pub fn exponential_generator (lambda : f64) -> f64
{
    let mut exp = Exp::new(lambda);
    let v = exp.sample(&mut rand::thread_rng());
    v
}

pub struct ConstantDistribution<T> {
    value: T
}

impl<T> Sample<T> for ConstantDistribution<T> {
    fn sample<R: Rng> (&mut self, _: &mut R) -> T {
        self.value
    }
}

impl<T> ConstantDistribution<T> {
    pub fn new(value: T) -> Self { ConstantDistribution { value } }
}
