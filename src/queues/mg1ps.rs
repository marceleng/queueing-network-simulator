extern crate rand;

use queues::request::Request;
use queues::Queue;
use float_binaryheap::FloatBinaryHeap;

use self::rand::distributions::Sample;

pub struct MG1PS<T> where T: Sample<f64> {
    time: f64,
    work_rate: f64,
    processes: FloatBinaryHeap<Request>,
    distribution: T,
}

impl<T> MG1PS<T> where T: Sample<f64> {
    pub fn new (work_rate: f64, distribution: T) -> MG1PS<T> {
        MG1PS {
            time: 0.,
            work_rate,
            processes: FloatBinaryHeap::new(),
            distribution
        }
    }
}

impl<T> Queue for MG1PS<T> where T: Sample<f64> {
    fn arrival (&mut self, req: Request) {
        let work = self.distribution.sample(&mut rand::thread_rng());
        self.processes.push(work, req)
    }

    fn update_time (&mut self, time: f64) {
        if self.processes.len() > 0 {
            let coef = self.work_rate / (self.processes.len() as f64);
            let work_update = (time - self.time) * coef;
            self.processes.translate_keys(-work_update);
        }
        self.time = time
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        match self.processes.peek() {
            None => None,
            Some((w,r)) => {
                Some((self.time + w*(self.processes.len() as f64)/self.work_rate, r))
            }
        }
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        match self.processes.pop() {
            None => None,
            Some((w,r)) => {
                Some((self.time + w*(self.processes.len() as f64)/self.work_rate, r))
            }
        }
    }
}
