use std::collections::VecDeque;
use queues::request::Request;
use queues::{Queue,Process};
use float_binaryheap::FloatBinaryHeap;

use distribution::MutDistribution;

pub struct MG1PS<T> where T: MutDistribution<f64> {
    time: f64,
    work_rate: f64,
    processes: VecDeque<Process>,
    distribution: T,
}

impl<T> MG1PS<T> where T: MutDistribution<f64> {
    pub fn new (work_rate: f64, distribution: T) -> MG1PS<T> {
        MG1PS{
            time: 0.,
            work_rate,
            processes: VecDeque::new(),
            distribution
        }
    }
}

impl<T> Queue for MG1PS<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        let work = self.distribution.mut_sample(&mut rand::thread_rng());
        let mut idx = 0;
        while idx < self.processes.len() && self.processes[idx].work > work {
                idx += 1;
        }
        self.processes.insert(idx, Process { req, work });
    }

    fn update_time (&mut self, time: f64) {
        let coef = self.work_rate / (self.processes.len() as f64);
        let work_update = (time - self.time) * coef;
        for process in self.processes.iter_mut() {
            (*process).work -= work_update;
        }
        self.time = time
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        self.processes.back().map(|p| (self.time + p.work*(self.processes.len() as f64)/self.work_rate, &p.req))
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        self.processes.pop_back().map(|p| (self.time + p.work*(self.processes.len() as f64)/self.work_rate, p.req))
    }
}


pub struct Mg1psOld<T> where T: MutDistribution<f64> {
    time: f64,
    work_rate: f64,
    processes: FloatBinaryHeap<Request>,
    distribution: T,
}

impl<T> Mg1psOld<T> where T: MutDistribution<f64> {
    pub fn new (work_rate: f64, distribution: T) -> Mg1psOld<T> {
        Mg1psOld {
            time: 0.,
            work_rate,
            processes: FloatBinaryHeap::new(),
            distribution
        }
    }
}

impl<T> Queue for Mg1psOld<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        let work = self.distribution.mut_sample(&mut rand::thread_rng());
        self.processes.push(work, req)
    }

    fn update_time (&mut self, time: f64) {
        if !self.processes.is_empty()  {
            let coef = self.work_rate / (self.processes.len() as f64);
            let work_update = (time - self.time) * coef;
            self.processes.translate_keys(-work_update);
        }
        self.time = time
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        self.processes.peek().map(|(w,r)| (self.time + w*(self.processes.len() as f64)/self.work_rate, r))
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        self.processes.pop().map(|(w,r)| (self.time + w*(self.processes.len() as f64)/self.work_rate, r))
    }
}
