use std::collections::{HashMap,VecDeque};

use queues::request::Request;
use queues::Queue;
use float_binaryheap::FloatBinaryHeap;

use distribution::MutDistribution;

//#[derive(Clone)]
pub struct MG1PS<T,Process> where T: MutDistribution<f64>, Process: PartialEq {
    time: f64,
    work_rate: f64,
    processes: FloatBinaryHeap<Process>,
    applied_work: f64,
    distribution: T,
}

impl<T,Process> MG1PS<T,Process> where T: MutDistribution<f64>, Process: PartialEq {
    pub fn new (work_rate: f64, distribution: T) -> MG1PS<T,Process> {
        MG1PS{
            time: 0.,
            work_rate,
            processes: FloatBinaryHeap::new(),
            applied_work: 0.,
            distribution
        }
    }

    fn peek(&self) -> Option<(f64, &Process)> {
        self.processes.peek().map(|(w,r)| (self.time + (w-self.applied_work) / self.work_rate * self.processes.len() as f64, r))
    }

    fn pop(&mut self) -> Option<(f64,Process)> {
        let nb_processes = self.processes.len() as f64;
        self.processes.pop().map(|(w,r)| (self.time + (w-self.applied_work) / self.work_rate * nb_processes, r))
    }

    fn insert_process (&mut self, proc: Process) {
        let work_target = self.distribution.mut_sample(&mut rand::thread_rng()) + self.applied_work;
        self.processes.push(work_target, proc);
    }

    fn advance_time (&mut self, time: f64) {
        if !self.processes.is_empty() {
            self.applied_work += (time-self.time) * self.work_rate / (self.processes.len() as f64);
        }

        self.time = time
    }
}

impl<T> Queue for MG1PS<T,Request> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        self.insert_process(req);
    }

    fn update_time (&mut self, time: f64) {
        self.advance_time(time);
    }

    fn read_next_exit(&self) -> Option<(f64, &Request)> {
        self.peek()
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        self.pop()
    }

    fn read_load (&self) -> usize {
        self.processes.len()
    }
}

pub struct AggregatingMG1PS<T> where T: MutDistribution<f64> {
    queue: MG1PS<T, usize>,
    pit: HashMap<usize, VecDeque<Request>>,
    to_release: VecDeque<Request>,
    load: usize
}

impl<T> AggregatingMG1PS<T> where T: MutDistribution<f64> {
    pub fn new(work_rate: f64, distribution: T) -> Self {
        AggregatingMG1PS {
            queue: MG1PS::new(work_rate, distribution),
            pit: HashMap::new(),
            to_release: VecDeque::new(),
            load: 0,
        }
    }
}

impl<T> Queue for AggregatingMG1PS<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {
        let content = req.get_content();
        let to_aggregate = self.pit.contains_key(&content) && !self.pit.get(&content).unwrap().is_empty();

        if to_aggregate {
            self.pit.get_mut(&content).unwrap().push_back(req);
        }
        else {
            let mut v = VecDeque::new();
            v.push_back(req);
            self.pit.insert(content,v);
            self.queue.insert_process(content);
        }
        self.load += 1;
    }

    fn update_time (&mut self, time: f64) {
        self.queue.advance_time(time);
    }

    fn read_next_exit (&self) -> Option<(f64, &Request)> {
        if !self.to_release.is_empty() {
            Some((self.queue.time, self.to_release.front().unwrap()))
        }
        else {
            self.queue.peek().map(|(t,c)| (t,self.pit.get(c).unwrap().front().unwrap()))
        }
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)> {
        if !self.to_release.is_empty() {
            self.load -= 1;
            Some((self.queue.time, self.to_release.pop_front().unwrap()))
        }
        else if let Some((t,c)) = self.queue.pop() {
            self.load -= 1;
            self.to_release = self.pit.remove(&c).unwrap();
            let req = self.to_release.pop_front().unwrap();

            Some((t,req))
        }
        else {
            None
        }
    }

    fn read_load (&self) -> usize {
        self.load
    }

}
