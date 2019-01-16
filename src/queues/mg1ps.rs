use queues::request::Request;
use queues::Queue;
use float_binaryheap::FloatBinaryHeap;

use distribution::MutDistribution;

//#[derive(Clone)]
pub struct MG1PS<T> where T: MutDistribution<f64> {
    time: f64,
    work_rate: f64,
    processes: FloatBinaryHeap<Request>,
    applied_work: f64,
    distribution: T,
}

impl<T> MG1PS<T> where T: MutDistribution<f64> {
    pub fn new (work_rate: f64, distribution: T) -> MG1PS<T> {
        MG1PS{
            time: 0.,
            work_rate,
            processes: FloatBinaryHeap::new(),
            applied_work: 0.,
            distribution
        }
    }
}

impl<T> Queue for MG1PS<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        let work_target = self.distribution.mut_sample(&mut rand::thread_rng()) + self.applied_work;

        self.processes.push(work_target, req);
    }

    fn update_time (&mut self, time: f64) {
        if !self.processes.is_empty() {
            self.applied_work += (time-self.time) * self.work_rate / (self.processes.len() as f64);
        }

        self.time = time
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        self.processes.peek().map(|(w,r)| (self.time + (w-self.applied_work) / self.work_rate * self.processes.len() as f64, r))
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        let nb_processes = self.processes.len() as f64;
        self.processes.pop().map(|(w,r)| (self.time + (w-self.applied_work) / self.work_rate * nb_processes, r))
    }

    fn read_load (&self) -> usize {
        self.processes.len()
    }
}
