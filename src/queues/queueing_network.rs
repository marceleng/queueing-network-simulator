use queues::Queue;
use queues::request::Request;
use std::vec::Vec;
use std::f64::INFINITY;

type TransitionFunc = Box<Fn(&Request, &QNet)->usize>;


#[derive(Debug)]
pub struct Transition {
    pub time: f64,
    pub origin: usize,
    pub destination: usize
}

#[derive(Debug)]
pub enum TransitionError {
    NoExitFound(usize),
    NoTransitionFound(usize),
    UnknownError
}

pub struct QNet {
    pub number_of_queues: usize,
    pub queues: Vec<Box<Queue>>,
    pub transitions: Vec<Option<TransitionFunc>>,
    pub time: f64
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

    pub fn add_transition(&mut self, queue: usize, trans: TransitionFunc)
    {
        self.transitions[queue] = Some(trans);
    }

    pub fn get_queue_mut(&mut self, queue: usize) -> &mut Queue
    {
        &mut *(self.queues[queue])
    }

    pub fn get_queue(&self, queue: usize) -> &Queue
    {
        & (*self.queues[queue])
    }    

    pub fn change_queue(&mut self, queue: usize, q: Box<Queue>) -> usize
    {
        self.queues[queue] = q;
        queue
    } 

    pub fn make_transition (&mut self) -> Result<Transition,TransitionError>
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
            self.queues.iter_mut().for_each(|x| x.update_time(next_exit));

            if let Some((t,mut r)) = self.queues[orig_q].pop_next_exit() {
                self.time = t;
                match self.transitions[orig_q] {
                    None => Err(TransitionError::NoTransitionFound(orig_q)),
                    Some(ref f) => { 
                        let dest_q = f(&r, &self);
                        r.add_log_entry(t, (orig_q, dest_q));
                        //println!("Transition: {}->{}", orig_q, dest_q);
                        self.queues[dest_q].arrival(r);
                        Ok(Transition {
                            time: t,
                            origin: orig_q,
                            destination: dest_q
                        })
                    }
                }
            }
            else {
                Err(TransitionError::UnknownError)
            }
        }
        else {
            Err(TransitionError::NoExitFound(orig_q))
        }
    }

    pub fn get_time(&self) -> f64 {
        self.time
    }
}
