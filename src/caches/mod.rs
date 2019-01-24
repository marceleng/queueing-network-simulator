extern crate rand;

pub mod lru_cache;
pub mod abf_cache;
pub mod abf_fpga_cache;

use rand::Rng;
use std::f64;

pub trait Cache<T> {
    fn contains (&mut self, entry: &T) -> bool;
    fn update (&mut self, entry: T);
}

pub struct RandomAccept(f64);

impl RandomAccept {
    pub fn from_value(f: f64) -> Result<RandomAccept,()> {
        if 0. <= f && f <= 1. {
            Ok(RandomAccept { 0: f })
        }
        else {
            Err(())
        }
    }
}

impl<T> Cache<T> for RandomAccept {
    fn contains (&mut self, _: &T) -> bool {
        rand::thread_rng().gen_range(0.,1.) < self.0
    }

    fn update(&mut self, _: T) {}
}

pub struct PerfectLfu(usize);

impl PerfectLfu {
    pub fn new(klfu: usize) -> Self {
        PerfectLfu { 0: klfu }
    }
}

impl Cache<usize> for PerfectLfu {
    fn contains(&mut self, entry: &usize) -> bool {
        entry <= &self.0
    }

    fn update(&mut self, _: usize) {}
}
