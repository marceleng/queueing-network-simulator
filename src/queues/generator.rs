extern crate rand;

use distribution::MutDistribution;
use queues::Queue;
use queues::request::Request;
use std::mem;

pub struct Generator<T> where T: MutDistribution<f64> {
    iat_distribution: T,
    time: f64,
    next_req: Request,
    next_arrival: f64
}

impl<T> Generator<T> where T: MutDistribution<f64> {
    pub fn new(iat_distribution: T) -> Self {
        let mut ret = Generator {
            iat_distribution,
            time: 0.,
            next_req: Request::new(0),
            next_arrival: 0.
        };
        ret.draw_arrival();

        ret
    }

    fn draw_arrival(&mut self) {
        self.next_arrival = self.time + self.iat_distribution.mut_sample(&mut rand::thread_rng());
    }
}

impl<T> Queue for Generator<T> where T: MutDistribution<f64> {
    fn arrival(&mut self, _req: Request) {
        panic!("You should not arrive at a generator");
    }

    fn update_time(&mut self, time: f64) {
        self.time = time;
    }

    fn read_next_exit (&self) -> Option<(f64, &Request)> {
        Some((self.next_arrival, &self.next_req))
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)> {
        let mut req = Request::new(0);
        mem::swap(&mut req, &mut self.next_req);
        let arrival = self.next_arrival;
        self.draw_arrival();
        Some((arrival, req))
    }
}
