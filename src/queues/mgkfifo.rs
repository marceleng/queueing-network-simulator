use float_binaryheap::FloatBinaryHeap;
use distribution::MutDistribution;
use std::collections::VecDeque;
use queues::request::Request;

use queues::{Queue,Process};

pub struct MGKFIFO<T> where T: MutDistribution<f64> {
    time: f64,
    work_rate: f64,
    queue: VecDeque<Process>,
    servers: Vec<Option<Process>>,
    free_servers: Vec<usize>,
    next_exits: FloatBinaryHeap<usize>,
    distribution: T,
}

impl<T> MGKFIFO<T> where T: MutDistribution<f64> {
    pub fn new(k: usize, work_rate: f64, distribution: T) -> Self {
        MGKFIFO {
            time: 0.,
            work_rate,
            queue: VecDeque::new(),
            servers: vec![None; k],
            free_servers: Vec::new(),
            next_exits: FloatBinaryHeap::new(),
            distribution
        }
    }

    //Sets 'process' as the active job in 'server'
    fn enqueue(&mut self, server: usize, process: Process) {
        assert!(self.servers[server].is_none());

        self.next_exits.push(self.time + process.work / self.work_rate, server);
        self.servers[server] = Some(process);
    }

    //Returns the active process in 'server'
    fn exit(&mut self, server: usize) -> Option<Request> {
        let ret = self.servers[server].take().map(|x| x.req);

        if let Some(process) = self.queue.pop_front() {
            self.enqueue(server, process);
        }
        else {
            self.free_servers.push(server);
        }

        ret
    }
}


impl<T> Queue for MGKFIFO<T> where T: MutDistribution<f64> {
    fn arrival (&mut self, req: Request) {

        let process = Process {
            req,
            work: self.distribution.mut_sample(&mut rand::thread_rng())
        };

        if let Some(server) = self.free_servers.pop() {
            self.enqueue(server, process)
        }
        else {
            self.queue.push_back(process)
        }
    }

    fn update_time    (&mut self, time: f64) {
        self.time = time;
    }

    fn read_next_exit (&self) -> Option<(f64,&Request)>  {
        match self.next_exits.peek() {
            Some((t, &s)) => {
                Some((t, &(self.servers[s].as_ref().unwrap().req)))
            },
            None => None,
        }
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
    
        match self.next_exits.pop() {
            Some((t, s)) => {
                Some((t, self.servers[s].take().unwrap().req))
            },
            None => None
        }
    }
}
