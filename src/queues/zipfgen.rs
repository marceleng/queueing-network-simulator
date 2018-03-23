extern crate rand;

use queues::Queue;
use queues::request::Request;
use self::rand::distributions::Sample;
use std::vec::Vec;
use float_binaryheap::FloatBinaryHeap;

pub struct ZipfGenerator<T> where T: Sample<f64> {
    alpha: f64,
    catalogue_size: usize,
    popularity: Vec<f64>,
    next_arrivals: FloatBinaryHeap<Request>,
    iat_distribution: Vec<T>,
} 

impl<T> ZipfGenerator<T> where T: Sample<f64> {
    pub fn new<F> (alpha: f64, catalogue_size: usize, iat_func: F) -> Self 
        where F: Fn(f64)->T {
        let mut ret = ZipfGenerator {
            alpha,
            catalogue_size,
            popularity: Vec::with_capacity(catalogue_size),
            next_arrivals: FloatBinaryHeap::new(),
            iat_distribution: Vec::with_capacity(catalogue_size)
        };
        let mut count = 0.;
        for i in 0..catalogue_size {
            let pop: f64 = ((i+1) as f64).powf(-alpha);
            ret.popularity.push(pop);
            count += pop;
        }
        for i in 0..catalogue_size {
            ret.popularity[i as usize] /= count;
            ret.iat_distribution.push(iat_func(ret.popularity[i as usize]));
            ret.insert_exit(i+1, 0.);
        }
        ret
    }

    fn insert_exit(&mut self, content: usize, time: f64) {
        let ntime = time + self.iat_distribution[content-1].sample(&mut rand::thread_rng());
        self.next_arrivals.push(ntime, Request::new(content as u64));
    }

    pub fn get_alpha (&self) -> f64 {
        self.alpha
    }
}

impl<T> Queue for ZipfGenerator<T> where T: Sample<f64> {
    fn arrival (&mut self, _req: Request) {
        panic!("You should not arrive at a generator");
    }

    fn update_time (&mut self, _time: f64) {}

    fn read_next_exit (&self) -> Option<(f64, &Request)> {
        self.next_arrivals.peek()
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)> {
        let ret = self.next_arrivals.pop();
        match ret {
            None => (),
            Some((t,ref r)) => self.insert_exit(r.get_content() as usize, t)
        };
        ret
    }
}
