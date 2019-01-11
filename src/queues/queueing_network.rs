use queues::Queue;
use queues::request::Request;
use std::vec::Vec;
use std::f64::INFINITY;

type Transition = Box<Fn(&Request, &QNet)->usize>;

pub struct QNet {
    number_of_queues: usize,
    queues: Vec<Box<Queue>>,
    transitions: Vec<Option<Transition>>,
    time: f64
}

impl QNet {

    pub fn new () -> QNet {
        QNet {
            number_of_queues: 0,
            queues : Vec::new(),
            transitions: Vec::new(),
            time: 0.,
        }
    }

    pub fn add_queue(&mut self, q: Box<Queue>) -> usize
    {
        self.number_of_queues += 1;
        self.queues.push(q);
        self.transitions.push(None);
        self.number_of_queues-1
    }

    pub fn add_transition(&mut self, queue: usize, trans: Transition)
    {
        self.transitions[queue] = Some(trans);
    }

    pub fn get_queue(&mut self, queue: usize) -> &mut Box<Queue>
    {
        &mut self.queues[queue]
    }

    pub fn read_queue(&self, queue: usize) -> &Box<Queue>
    {
        &self.queues[queue]
    }    

    pub fn make_transition (&mut self)
    {
        let mut orig_q = self.number_of_queues;
        let mut next_exit = INFINITY;
        for queue in 0..self.number_of_queues {
            if let Some((t,_)) = self.queues[queue].read_next_exit() {
                if t <= next_exit {
                    next_exit = t;
                    orig_q = queue;
                }
            }
        }


        if orig_q < self.number_of_queues {
            if let Some((t,mut r)) = self.queues[orig_q].pop_next_exit() {
                self.time = t;
                //TODO: figure out how to use iterator instead
                for queue in 0..self.number_of_queues {
                    self.queues[queue].update_time(t)
                }
                match self.transitions[orig_q] {
                    None => println!("{} exits at t={}", r.get_id(), t),
                    Some(ref f) => { 
                        let dest_q = f(&r, &self);
                        r.add_log_entry(t, (orig_q, dest_q));
                        println!("Transition: {}->{}", orig_q, dest_q);
                        self.queues[dest_q].arrival(r)
                    }
                }
            }
        }
    }

    pub fn get_time(&self) -> f64 {
        self.time
    }
}
