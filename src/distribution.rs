extern crate rand;

use distribution::rand::Rng;

use distribution::rand::distributions::{Exp, Distribution};

pub fn exponential_generator (lambda : f64) -> f64
{
    let mut exp = Exp::new(lambda);
    let v = exp.sample(&mut rand::thread_rng());
    v
}

pub struct ConstantDistribution<T> where T: Copy{
    value: T
}

impl<T> Distribution<T> for ConstantDistribution<T> where T: Copy {
    fn sample<R: Rng + ?Sized> (&self, _: &mut R) -> T {
        self.value
    }
}

impl<T> ConstantDistribution<T> where T: Copy {
    pub fn new(value: T) -> Self { ConstantDistribution { value } }
}

pub struct OffsetExp {
    exp: Exp,
    offset: f64
}

impl OffsetExp {
    pub fn new(offset: f64, lambda: f64) -> Self {
        OffsetExp {
            exp: Exp::new(lambda),
            offset
        }
    }
}

impl Distribution<f64> for OffsetExp {
    fn sample<R: Rng + ?Sized> (&self, r: &mut R) -> f64 {
        self.offset + self.exp.sample(r)
    }
}
