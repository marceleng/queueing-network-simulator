extern crate rand;

use rand::Rng;

use rand::distributions::{Exp, Distribution};

//Dummy trait to enable stateful distributions (eg, MMPP)
pub trait MutDistribution<T> {
    fn mut_sample<R:Rng + ?Sized> (&mut self, r: &mut R) -> T;
}

pub fn exponential_generator (lambda : f64) -> f64
{
    Exp::new(lambda).sample(&mut rand::thread_rng())
}

pub struct ConstantDistribution<T> where T: Copy{
    value: T
}

impl<S,T> MutDistribution<T> for S where S: Distribution<T> {
    fn mut_sample<R: Rng + ?Sized> (&mut self, r: &mut R)-> T {
        self.sample(r)
    }
}


impl<T> ConstantDistribution<T> where T: Copy {
    pub fn new(value: T) -> Self { ConstantDistribution { value } }
}

impl<T> Distribution<T> for ConstantDistribution<T> where T: Copy {
    fn sample<R: Rng + ?Sized> (&self, _: &mut R) -> T {
        self.value
    }
}

impl<T> Clone for ConstantDistribution<T> where T: Copy {
    fn clone(&self) -> Self {  ConstantDistribution { value: self.value }  }
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

pub struct MMPP2 {
    lambdas: Vec<Exp>,
    transitions: Vec<Exp>,
    current_state: usize,
    time_to_next_transition: f64,
}

impl MMPP2 {
    pub fn new(lambda1: f64, mu1: f64, lambda2: f64, mu2: f64) -> Self {
        MMPP2 {
            lambdas: vec![Exp::new(lambda1), Exp::new(lambda2)],
            transitions: vec![Exp::new(mu1), Exp::new(mu2)],
            current_state: 1,
            time_to_next_transition: 0.,
        }
    }
}

impl MutDistribution<f64> for MMPP2 {
    fn mut_sample<R: Rng + ?Sized> (&mut self, r: &mut R) -> f64 {
        let mut ret = self.lambdas[self.current_state].sample(r);

        //State transition
        while ret > self.time_to_next_transition {
            self.current_state = 1 - self.current_state;
            ret = self.time_to_next_transition + self.lambdas[self.current_state].sample(r);
            self.time_to_next_transition += self.transitions[self.current_state].sample(r);
        }

        self.time_to_next_transition -= ret;
        ret
    }
}
