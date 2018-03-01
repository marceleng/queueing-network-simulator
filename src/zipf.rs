extern crate rand;

use self::rand::distributions::Sample;
use self::rand::Rng;
use std::vec::Vec;

pub struct Zipf {
    alpha: f64,
    catalogue_size: usize,
    popularity: Vec<f64>,
} 

impl Zipf {
    pub fn new (alpha: f64, catalogue_size: usize) -> Self {
        let mut ret = Zipf {
            alpha,
            catalogue_size,
            popularity: Vec::with_capacity(catalogue_size)
        };
        let mut count = 0.;
        for i in 0..catalogue_size {
            let pop: f64 = ((i+1) as f64).powf(-alpha);
            ret.popularity.push(pop);
            count += pop;
        }
        for i in 0..catalogue_size {
            ret.popularity[i as usize] /= count;
        }
        ret
    }
}

impl Sample<u64> for Zipf {
    fn sample<R: Rng> (&mut self, rng: &mut R) -> u64 {
        let seed = rng.gen_range::<f64>(0.,1.);
        let mut count = 0.;
        let mut ret: usize = 0;

        while seed > count && ret < self.catalogue_size {
            count += self.popularity[ret];
            ret += 1;
        }
        ret as u64
    }
}
