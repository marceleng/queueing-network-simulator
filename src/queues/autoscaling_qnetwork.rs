use queues::queueing_network::QNet;
use queues::Queue;
use queues::request::Request;
use std::vec::Vec;
use std::f64::INFINITY;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use distribution::MutDistribution;

use rand::distributions::{Exp};
use distribution::{ConstantDistribution};

use rand::Rng;

enum ScalingOperation {
    NOOP,
    DOWNSCALING,
    UPSCALING
}
struct AutoscalingTracker {
    last_event_time: f64,
    num_events: usize,
    proba_empty_ewma: f64,
    ewma_window_len: f64,
    upscale_threshold: f64,
    downscale_threshold: f64,
    did_request_scaling: bool
}

impl AutoscalingTracker {
    fn new(_downscale_threshold: f64, _upscale_threshold: f64, _ewma_window_len: f64) -> Self {
        AutoscalingTracker {
            last_event_time: -1.,
            num_events: 0,
            proba_empty_ewma: 0.6,
            ewma_window_len: _ewma_window_len,
            upscale_threshold: _upscale_threshold,
            downscale_threshold: _downscale_threshold,
            did_request_scaling: false
        }
    }

    fn fact(n: f64) -> f64 
    {
        if (n.abs() < 0.1) || ((n - 1.).abs() < 0.1) { 1. } else { n * Self::fact(n - 1.) }
    }

    fn downscale_threshold(pe: f64, n: usize) -> f64
    {
        if n < 2 { return 1.; }
        let nm1 = n - 1;
        let nm2 = n - 2;
        let mut conv = roots::SimpleConvergency { eps:1e-15f64, max_iter:1000 };
        let rho = roots::find_root_brent(0., (2*n) as f64, |x| { let denom: f64 = (0..nm1).map(|k| { x.powf(k as f64) / (Self::fact(k as f64))}).sum(); (x.powf(nm1 as f64) / (Self::fact(nm2 as f64))) / denom - (1. - pe)}, &mut conv).unwrap();
        roots::find_root_brent(0., 1., |x| { let denom: f64 = (0..n).map(|k| { rho.powf(k as f64) / (Self::fact(k as f64))}).sum(); (rho.powf(n as f64) / (Self::fact(nm1 as f64))) / denom - (1. - x)}, &mut conv).unwrap()
    }

    fn upscale_threshold(pe: f64, n: usize) -> f64
    {
        let np1 = n + 1;
        let nm1 = n - 1;
        let mut conv = roots::SimpleConvergency { eps:1e-15f64, max_iter:1000 };
        let rho = roots::find_root_brent(0., (2*n) as f64, |x| { let denom: f64 = (0..np1).map(|k| { x.powf(k as f64) / (Self::fact(k as f64))}).sum(); (x.powf(np1 as f64) / (Self::fact(n as f64))) / denom - (1. - pe)}, &mut conv).unwrap();
        roots::find_root_brent(0., 1., |x| { let denom: f64 = (0..n).map(|k| { rho.powf(k as f64) / (Self::fact(k as f64))}).sum(); (rho.powf(n as f64) / (Self::fact(nm1 as f64))) / denom - (1. - x)}, &mut conv).unwrap()
    }

    fn update(&mut self, time: f64, load: usize) -> ScalingOperation
    {
        let alpha = if self.last_event_time > 0. { 1. - (-(time - self.last_event_time) / self.ewma_window_len).exp() } else { 0.01 };
        let incr = if load == 0 { 1. } else { 0. };
        self.proba_empty_ewma = (1. - alpha) * self.proba_empty_ewma + alpha * incr;
        //println!("time={}, last_time={}, load={}, ewma={}", time, self.last_event_time, load, self.proba_empty_ewma);
        self.last_event_time = time;
        self.num_events += 1;
        if self.proba_empty_ewma > self.downscale_threshold && self.num_events >= 50 && !self.did_request_scaling { 
            self.did_request_scaling = true;
            ScalingOperation::DOWNSCALING 
        } else if self.proba_empty_ewma < self.upscale_threshold && self.num_events >= 50 && !self.did_request_scaling { 
            self.did_request_scaling = true;           
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
    pnetwork_arcs: Vec<usize>,
    autoscaling_tracker: Option<AutoscalingTracker>,
    pserver_with_tracker: usize
}

impl AutoscalingQNet {
    pub fn new (traffic_source: Box<Queue>, file_logger: Box<Queue>) -> Self {
        let n = 0;
        let mut _qn = QNet::new();
        let _ptraffic_source = _qn.add_queue(traffic_source);
        let _pfile_logger = _qn.add_queue(file_logger);
        AutoscalingQNet {
            qn : _qn,
            n_servers: 0,
            ptraffic_source: _ptraffic_source,
            pfile_logger: _pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n],
            autoscaling_tracker: None,
            pserver_with_tracker: 0 as usize
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
                let mut dest_q : Option<usize> = None;
                match self.qn.transitions[orig_q] {
                    None => println!("{} exits at t={}", r.get_id(), t),
                    Some(ref f) => {
                        let _dest_q = f(&r, &self.qn);
                        r.add_log_entry(t, (orig_q, _dest_q));
                        //println!("Transition: {}->{}", orig_q, _dest_q);
                        self.qn.queues[_dest_q].arrival(r);
                        dest_q = Some(_dest_q);

                    }
                }
                let mut scaling_op = ScalingOperation::NOOP;
                let leave_event = self.autoscaling_tracker.is_some() && (orig_q == self.pserver_with_tracker);
                let mut arrival_event = false;
                if let Some(_dest_q) = dest_q {
                    if _dest_q == self.pserver_with_tracker && self.autoscaling_tracker.is_some() {
                        arrival_event = true;
                    }
                }

                let mut load_before_event = self.qn.queues[self.pserver_with_tracker].read_load();
                if leave_event {
                    load_before_event += 1;
                } else if arrival_event {
                    load_before_event -= 1;
                }

                if leave_event || arrival_event {
                    if let Some(ref mut tracker) = self.autoscaling_tracker {
                        scaling_op = tracker.update(t, load_before_event);
                    }
                }
                match scaling_op {
                    ScalingOperation::UPSCALING => { 
                        println!("t {} upscale_to {}", t, self.n_servers + 1); 
                        //self.add_server(ConstantDistribution::new(/*0.000_200*/0.000_000), /* Exp::new(mu)*/ConstantDistribution::new(1./10.)) 
                    },
                    ScalingOperation::DOWNSCALING => { 
                        println!("t{} downscale_to {}", t, self.n_servers - 1); 
                        //self.remove_server() 
                    },
                    ScalingOperation::NOOP => ()
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
        // Transition traffic_source -> link(src, server 0)
        } else if self.n_servers == 1 {
            let dest = self.pnetwork_arcs[0];
            self.qn.add_transition(self.ptraffic_source,  Box::new(move |_,_| dest ));
        } else {
            // If there are no servers, sink the traffic source to /dev/null
            let dest = self.pfile_logger;
            self.qn.add_transition(self.ptraffic_source,  Box::new(move |_,_| dest ));
        }

        if self.n_servers > 0 {
            // Transition link({server n-1 or src}, server n) -> { server n }
            {
                let source = self.pnetwork_arcs[last_server_idx];
                let potential_dest = self.pservers[last_server_idx];
                let JIQ = false;
                if JIQ {
                    let fallback_dests = self.pservers.clone();  
                    self.qn.add_transition(source, Box::new(move |ref _req, ref qn| { 
                        let load = qn.get_queue(potential_dest).read_load();
                        if load == 0 { potential_dest } else { *rand::thread_rng().choose(&fallback_dests).unwrap() }
                    }));
                } else {
                    self.qn.add_transition(source, Box::new(move |_,_| potential_dest ));
                }
            }

            // Transition server n -> file_logger
            {
                let source = self.pservers[last_server_idx];
                let dest = self.pfile_logger;
                self.qn.add_transition(source, Box::new(move |_,_| dest ));
            }
        }
    }

    fn do_autoscale(&mut self)
    {
        let pe = 0.8;
        let downscale_threshold = AutoscalingTracker::downscale_threshold(pe, self.n_servers);
        let upscale_threshold = AutoscalingTracker::upscale_threshold(pe, self.n_servers); 
        let ewma_len = 30.; //FIXME take 300 times E[service time]
        self.autoscaling_tracker = Some(AutoscalingTracker::new(downscale_threshold, upscale_threshold, ewma_len));     
        self.pserver_with_tracker = self.pservers[self.n_servers - 1];  
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

        self.do_autoscale();
        self.update_network();   

        //println!("n_servers = {}", self.n_servers);
    }

    pub fn remove_server(&mut self) {
        // We don't actually perform removal of the queues, so that they can flush naturally
        // However, we update the network so that no transition points to those queues

        if self.n_servers > 0 {
            self.n_servers -= 1;

            if self.n_servers > 0 {
                self.do_autoscale();    
            }

            self.update_network();
        }
        //println!("n_servers = {}", self.n_servers);

    }
}
