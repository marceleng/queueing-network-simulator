use queues::queueing_network::{QNet,Transition,TransitionError};
use queues::Queue;
use std::vec::Vec;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use helpers::distribution::MutDistribution;
use helpers::ewma::TimeWindowedEwma;

enum ScalingOperation {
    NOOP,
    DOWNSCALING,
    UPSCALING
}
pub struct AutoscalingParameters {
    pub proba_empty: f64,
    pub ewma_window_len: f64
}
struct AutoscalingTracker {
    ewma: TimeWindowedEwma,
    num_events: usize,
    num_events_to_converge: usize,
    upscale_threshold: f64,
    downscale_threshold: f64,
    did_request_scaling: bool
}

impl AutoscalingTracker {
    fn new(downscale_threshold: f64, pe_start: f64, upscale_threshold: f64, ewma_window_len: f64) -> Self {
        AutoscalingTracker {
            ewma: TimeWindowedEwma::from_initial_value(pe_start, ewma_window_len),
            num_events: 0,
            num_events_to_converge: 100,
            upscale_threshold,
            downscale_threshold,
            did_request_scaling: false,
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
        let proba_empty_ewma = self.ewma.update(time, if load == 0 { 1. } else { 0. });
        //println!("time={}, last_time={}, load={}, ewma={}", time, self.last_event_time, load, self.proba_empty_ewma);
        self.num_events += 1;
        if proba_empty_ewma > self.downscale_threshold && self.num_events >= self.num_events_to_converge && !self.did_request_scaling {
            self.did_request_scaling = true;
            ScalingOperation::DOWNSCALING
        } else if proba_empty_ewma < self.upscale_threshold && self.num_events >= self.num_events_to_converge && !self.did_request_scaling {
            self.did_request_scaling = true;
            ScalingOperation::UPSCALING
        } else {
            ScalingOperation::NOOP
        }
    }
}


pub struct AutoscalingQNet<T1: 'static+ MutDistribution<f64>+Clone,T2: 'static+ MutDistribution<f64>+Clone> {
    qn: QNet,
    n_servers: usize,
    ptraffic_source: usize,
    pfile_logger: usize,
    pservers: Vec<usize>,
    pnetwork_arcs: Vec<usize>,
    autoscaling_parameters: Option<AutoscalingParameters>,    
    autoscaling_tracker: Option<AutoscalingTracker>,
    pserver_with_tracker: usize,
    link_distribution: T1,
    server_distribution: T2,
}

impl<T1,T2> AutoscalingQNet<T1,T2> where T1:MutDistribution<f64>+Clone, T2:MutDistribution<f64>+Clone {
    pub fn new (traffic_source: Box<Queue>,
                file_logger: Box<Queue>,
                n_servers: usize,
                link_distribution: T1,
                server_distribution: T2,
                autoscaling_parameters: Option<AutoscalingParameters>) -> Self {
        let n = 0;
        let mut qn = QNet::new();
        let ptraffic_source = qn.add_queue(traffic_source);
        let pfile_logger = qn.add_queue(file_logger);
        let mut ret = AutoscalingQNet {
            qn,
            n_servers: 0,
            ptraffic_source,
            pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n],
            autoscaling_parameters,
            autoscaling_tracker: None,
            pserver_with_tracker: 0 as usize,
            link_distribution,
            server_distribution,
        };
        for _i in 0..n_servers {
            ret.add_server();
        }
        ret

    }

    pub fn make_transition (&mut self) -> Result<Transition, TransitionError>
    {
        let trans = self.qn.make_transition()?;

        let mut scaling_op = ScalingOperation::NOOP;
        let leave_event = self.autoscaling_tracker.is_some() && (trans.origin == self.pserver_with_tracker);
        let arrival_event = trans.destination == self.pserver_with_tracker && self.autoscaling_tracker.is_some();

        let mut load_before_event = self.qn.queues[self.pserver_with_tracker].read_load();
        if leave_event {
            load_before_event += 1;
        } else if arrival_event {
            load_before_event -= 1;
        }

        if leave_event || arrival_event {
            if let Some(ref mut tracker) = self.autoscaling_tracker {
                scaling_op = tracker.update(trans.time, load_before_event);
            }
        }
        match scaling_op {
            ScalingOperation::UPSCALING => {
                println!("t {} upscale_to {}", trans.time, self.n_servers + 1);
                self.add_server()
            },
            ScalingOperation::DOWNSCALING => {
                println!("t {} downscale_to {}", trans.time, self.n_servers - 1);
                self.remove_server()
            },
            ScalingOperation::NOOP => ()
        }

        Ok(trans)
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
                /*
                let JIQ = false;
                if JIQ {
                    let fallback_dests = self.pservers.clone();
                    self.qn.add_transition(source, Box::new(move |ref _req, ref qn| {
                        let load = qn.get_queue(potential_dest).read_load();
                        if load == 0 { potential_dest } else { *rand::thread_rng().choose(&fallback_dests).unwrap() }
                    }));
                } else {
                    self.qn.add_transition(source, Box::new(move |_,_| potential_dest ));
                } */
                self.qn.add_transition(source, Box::new(move |_,_| potential_dest ));
            }

            // Transition server n -> file_logger
            {
                let source = self.pservers[last_server_idx];
                let dest = self.pfile_logger;
                self.qn.add_transition(source, Box::new(move |_,_| dest ));
            }
        }
    }

    fn setup_autoscale(&mut self)
    {
        if let Some(ref autoscaling_parameters) = self.autoscaling_parameters {
            let pe = autoscaling_parameters.proba_empty;
            let downscale_threshold = AutoscalingTracker::downscale_threshold(pe, self.n_servers);
            let upscale_threshold = AutoscalingTracker::upscale_threshold(pe, self.n_servers);
            self.autoscaling_tracker = Some(AutoscalingTracker::new(downscale_threshold, pe, upscale_threshold, autoscaling_parameters.ewma_window_len));
            self.pserver_with_tracker = self.pservers[self.n_servers - 1];
        }
    }

    fn add_server(&mut self)
    {
        if self.pservers.len() != self.pnetwork_arcs.len() {
            panic!("|pserver| != |pnetwork_arcs| while adding server!");
        }

        self.n_servers += 1;

        if self.pservers.len() < self.n_servers {
            /* Add new slot in vector if not existing */
            // Link { server n-1 [or src] } -> server n
            self.pnetwork_arcs.push(self.qn.add_queue(Box::new(MGINF::new(1., self.link_distribution.clone()))));
            // Server n
            self.pservers.push(self.qn.add_queue(Box::new(MG1PS::new(1., self.server_distribution.clone()))));

            if self.pservers.len() != self.n_servers {
                panic!("|pserver| != n_servers after adding server!");
            }
        } else {
            /* Reuse slot in vector if already existing */
            // Link { server n-1 [or src] } -> server n
            self.qn.change_queue(self.pnetwork_arcs[self.n_servers - 1],
                                     Box::new(MGINF::new(1., self.link_distribution.clone())));
            // Server n
            self.qn.change_queue(self.pservers[self.n_servers - 1],
                                     Box::new(MG1PS::new(1., self.server_distribution.clone())));
        }

        self.setup_autoscale();
        self.update_network();

        //println!("n_servers = {}", self.n_servers);
    }

    fn remove_server(&mut self) {
        // We don't actually perform removal of the queues, so that they can flush naturally
        // However, we update the network so that no transition points to those queues

        if self.n_servers > 0 {
            self.n_servers -= 1;

            if self.n_servers > 0 {
                self.setup_autoscale();
            }

            self.update_network();
        }
        //println!("n_servers = {}", self.n_servers);

    }
}
