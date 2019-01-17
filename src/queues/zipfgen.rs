extern crate rand;
extern crate zipf;

use distribution::MutDistribution;
use zipf::ZipfDistribution;

use std::vec::Vec;
use std::mem;

use queues::Queue;
use queues::request::Request;
use float_binaryheap::FloatBinaryHeap;

pub struct ZipfGenerator<T> where T: MutDistribution<f64> {
    pop_distribution: ZipfDistribution,
    ita_distribution: T,
    next_req: Request,
    next_arrival: f64,
    total_nb_arrivals: usize,
    cur_nb_arrivals: usize,
}

impl<T> ZipfGenerator<T> where T: MutDistribution<f64> {
    pub fn new (alpha: f64, catalogue_size: usize, distribution: T, total_nb_arrivals: usize) -> Self {
        let mut ret = ZipfGenerator {
            pop_distribution: ZipfDistribution::new(catalogue_size, alpha).unwrap(),
            ita_distribution: distribution,
            next_req: Request::new(0),
            next_arrival: 0.,
            total_nb_arrivals,
            cur_nb_arrivals: 0,
        };
        ret.draw_req();
        ret.draw_arrival();

        ret
    }

    fn draw_req (&mut self) -> Request {
        let mut new_req = Request::new(self.pop_distribution.mut_sample(&mut rand::thread_rng()));
        mem::swap(&mut self.next_req, &mut new_req);
        new_req
    }

    fn draw_arrival (&mut self) -> f64 {
        let ret = self.next_arrival;
        self.next_arrival += self.ita_distribution.mut_sample(&mut rand::thread_rng());
        ret
    }
}

impl<T> Queue for ZipfGenerator<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, _req: Request) {
        panic!("You should not arrive at a generator");
    }

    fn update_time (&mut self, _time: f64) {}

    fn read_next_exit (&self) -> Option<(f64, &Request)> {
        if self.total_nb_arrivals > self.cur_nb_arrivals {
            Some((self.next_arrival,&self.next_req))
        }
        else {
            None
        }
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)> {
        if self.cur_nb_arrivals < self.total_nb_arrivals {
            let req = self.draw_req();
            let arrival = self.draw_arrival();
            self.cur_nb_arrivals += 1;
            Some((arrival, req))
        }
        else {
            None
        }
    }

    fn read_load (&self) -> usize {
        1
    }        
}

pub struct ZipfGeneratorOld<T> where T: MutDistribution<f64> {
    alpha: f64,
    catalogue_size: usize,
    popularity: Vec<f64>,
    next_arrivals: FloatBinaryHeap<Request>,
    iat_distribution: Vec<T>,
} 

impl<T> ZipfGeneratorOld<T> where T: MutDistribution<f64> {
    pub fn new<F> (alpha: f64, catalogue_size: usize, iat_func: F) -> Self 
        where F: Fn(f64)->T
    {
        let mut ret = ZipfGeneratorOld {
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
        let ntime = time + self.iat_distribution[content-1].mut_sample(&mut rand::thread_rng());
        self.next_arrivals.push(ntime, Request::new(content));
    }

    pub fn get_alpha (&self) -> f64 {
        self.alpha
    }
}

impl<T> Queue for ZipfGeneratorOld<T> where T: MutDistribution<f64> {
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

    fn read_load (&self) -> usize {
        1
    }    
}
