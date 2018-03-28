extern crate rand;

use distribution::rand::Rng;

use distribution::rand::distributions::{Exp, Sample};

pub fn exponential_generator (lambda : f64) -> f64
{
    let mut exp = Exp::new(lambda);
    let v = exp.sample(&mut rand::thread_rng());
    v
}

pub struct ConstantDistribution<T> where T: Copy{
    value: T
}

impl<T> Sample<T> for ConstantDistribution<T> where T: Copy {
    fn sample<R: Rng> (&mut self, _: &mut R) -> T {
        self.value
    }
}

impl<T> ConstantDistribution<T> where T: Copy {
    pub fn new(value: T) -> Self { ConstantDistribution { value } }
}
