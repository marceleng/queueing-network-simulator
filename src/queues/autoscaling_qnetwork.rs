use queues::queueing_network::QNet;
use queues::Queue;
use queues::request::Request;
use std::vec::Vec;
use std::f64::INFINITY;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use distribution::MutDistribution;


enum ScalingOperation {
    NOOP,
    DOWNSCALING,
    UPSCALING
}
struct AutoscalingTracker {
    last_event_time: f64,
    num_events: usize,
    proba_empty_ewma: f64,
    ewma_window_len: f64
}

impl AutoscalingTracker {
    fn new(_ewma_window_len: f64) -> Self {
        AutoscalingTracker {
            last_event_time: 0.,
            num_events: 0,
            proba_empty_ewma: 0.,
            ewma_window_len: _ewma_window_len
        }
    }

    // returns true iff autoscaling needed
    fn update(&mut self, time: f64, load: usize) -> ScalingOperation
    {
        let alpha = 1. - (-(time - self.last_event_time) / self.ewma_window_len).exp();
        let incr = if load == 0 { 1. } else { 0. };
        self.proba_empty_ewma = (1. - alpha) * self.proba_empty_ewma + alpha * incr;
        self.last_event_time = time;
        self.num_events += 1;
        if self.proba_empty_ewma > 0.6 && self.num_events >= 50 { 
            ScalingOperation::DOWNSCALING 
        } else if self.proba_empty_ewma < 0.4 && self.num_events >= 50 { 
            ScalingOperation::UPSCALING 
        } else { 
            ScalingOperation::NOOP 
        }
    }
}


type Transition = Box<Fn(&Request, &QNet)->usize>;

pub struct AutoscalingQNet {
    qn: QNet,
    n_servers: usize,
    ptraffic_source: usize,
    pfile_logger: usize,
    pservers: Vec<usize>,
    pnetwork_arcs: Vec<usize>
}

impl AutoscalingQNet {    
    pub fn new (traffic_source: Box<Queue>, file_logger: Box<Queue>) -> Self {
        let n = 5;
        let mut _qn = QNet::new();
        let _ptraffic_source = _qn.add_queue(traffic_source);
        let _pfile_logger = _qn.add_queue(file_logger);        
        AutoscalingQNet {
            qn : _qn,
            n_servers: n,
            ptraffic_source: _ptraffic_source,
            pfile_logger: _pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n]
        }
    }

    pub fn make_transition (&mut self)
    {
        let mut orig_q = self.qn.number_of_queues;
        let mut next_exit = INFINITY;
        for queue in 0..self.qn.number_of_queues {
            if let Some((t,_)) = self.qn.queues[queue].read_next_exit() {
                if t <= next_exit {
                    next_exit = t;
                    orig_q = queue;
                }
            }
        }


        if orig_q < self.qn.number_of_queues {
            if let Some((t,mut r)) = self.qn.queues[orig_q].pop_next_exit() {
                self.qn.time = t;
                //TODO: figure out how to use iterator instead
                for queue in 0..self.qn.number_of_queues {
                    self.qn.queues[queue].update_time(t)
                }
                match self.qn.transitions[orig_q] {
                    None => println!("{} exits at t={}", r.get_id(), t),
                    Some(ref f) => { 
                        let dest_q = f(&r, &self.qn);
                        r.add_log_entry(t, (orig_q, dest_q));
                        //println!("Transition: {}->{}", orig_q, dest_q);
                        self.qn.queues[dest_q].arrival(r)
                    }
                }
            }
        }
    }

    fn update_network(&mut self) 
    {
        let last_server_idx = self.n_servers - 1;

        // Transition link({ server n-2 or src }, server n-1) -> { server(n-1) or link(server n-1, server n) }
        if self.n_servers >= 2 {
            let second_to_last_server_idx = self.n_servers - 2;                  
            let source = self.pnetwork_arcs[second_to_last_server_idx];
            let potential_dest = self.pservers[second_to_last_server_idx];
            let fallback_dest = self.pnetwork_arcs[last_server_idx];
            self.qn.add_transition(source, Box::new(move |ref _req, ref qn| { 
                let load = qn.get_queue(potential_dest).read_load();
                if load == 0 { potential_dest } else { fallback_dest }
            }));
        // Transition traffic_source -> server 0           
        } else if self.n_servers == 1 {
            let dest = self.pservers[0];
            self.qn.add_transition(self.ptraffic_source,  Box::new(move |_,_| dest ));
        } else {
            // If there are no servers, sink the traffic source to /dev/null
            let dest = self.pfile_logger;                       
            self.qn.add_transition(self.ptraffic_source,  Box::new(move |_,_| dest ));
        }

        // Transition link(server n-1, server n) -> { server n }
        {
            let source = self.pnetwork_arcs[last_server_idx];
            let dest = self.pservers[last_server_idx];
            self.qn.add_transition(source, Box::new(move |_,_| dest ));
        }

        // Transition server n -> file_logger
        {
            let source = self.pservers[last_server_idx];
            let dest = self.pfile_logger;           
            self.qn.add_transition(source, Box::new(move |_,_| dest ));
        }             
    }

    pub fn add_server<T1: 'static+ MutDistribution<f64>,T2: 'static+ MutDistribution<f64>>(&mut self, link_distribution: T1, server_distribution: T2)
    {
        if self.pservers.len() != self.pnetwork_arcs.len() {
            panic!("|pserver| != |pnetwork_arcs| while adding server!");
        }

        self.n_servers += 1;

        if self.pservers.len() < self.n_servers {
            /* Add new slot in vector if not existing */            
            // Link { server n-1 [or src] } -> server n
            self.pnetwork_arcs.push(self.qn.add_queue(Box::new(MGINF::new(1., link_distribution))));
            // Server n
            self.pservers.push(self.qn.add_queue(Box::new(MG1PS::new(1., server_distribution))));

            if self.pservers.len() != self.n_servers {
                panic!("|pserver| != n_servers after adding server!");
            }            
        } else {
            /* Reuse slot in vector if already existing */
            // Link { server n-1 [or src] } -> server n           
            self.pnetwork_arcs[self.n_servers - 1] = self.qn.change_queue(self.pnetwork_arcs[self.n_servers - 1], Box::new(MGINF::new(1., link_distribution)));
            // Server n            
            self.pservers[self.n_servers - 1] = self.qn.change_queue(self.pservers[self.n_servers - 1], Box::new(MG1PS::new(1., server_distribution)));
        }

        self.update_network();   
    }

    pub fn remove_server(&mut self) {
        // We don't actually perform removal of the queues, so that they can flush naturally
        // However, we update the network so that no transition points to those queues

        if self.n_servers > 0 {
            self.n_servers -= 1;
            self.update_network();
        }
    } 
}