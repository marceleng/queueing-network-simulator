extern crate rand;

use distribution::rand::Rng;

use distribution::rand::distributions::{Exp, Sample};

pub fn exponential_generator (lambda : f64) -> f64
{
    let mut exp = Exp::new(lambda);
    let v = exp.sample(&mut rand::thread_rng());
    v
}
