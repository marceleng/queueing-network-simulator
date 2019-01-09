use queues::request::Request;
use queues::Queue;
use float_binaryheap::FloatBinaryHeap;

use distribution::MutDistribution;

pub struct MGINF<T> where T: MutDistribution<f64> {
    time: f64,
    work_rate: f64,
    processes: FloatBinaryHeap<Request>,
    distribution: T,
}

impl<T> MGINF<T> where T: MutDistribution<f64> {
    pub fn new (work_rate: f64, distribution: T) -> MGINF<T> {
        MGINF {
            time: 0.,
            work_rate,
            processes: FloatBinaryHeap::new(),
            distribution
        }
    }
}

impl<T> Queue for MGINF<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        let exit = self.distribution.mut_sample(&mut rand::thread_rng()) / self.work_rate + self.time;
        self.processes.push(exit, req)
    }

    fn update_time (&mut self, time: f64) {
        self.time = time
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        self.processes.peek()
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        self.processes.pop()
    }
}
