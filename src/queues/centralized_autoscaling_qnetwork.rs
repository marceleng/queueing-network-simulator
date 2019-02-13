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


type Transition = Box<Fn(&Request, &QNet)->usize>;

pub enum CentralizedLBPolicy {
    RND,
    JSQ2,
    JIQ
}

pub struct CentralizedAutoscalingQNet<T1: 'static+ MutDistribution<f64>+Clone,T2: 'static+ MutDistribution<f64>+Clone> {
    qn: QNet,
    n_servers: usize,
    ptraffic_source: usize,
    pfile_logger: usize,
    pservers: Vec<usize>,
    pnetwork_arcs: Vec<usize>,
    lb_policy: CentralizedLBPolicy,
    link_distribution: T1,
    server_distribution: T2
}

impl<T1,T2> CentralizedAutoscalingQNet<T1,T2> where T1:MutDistribution<f64>+Clone, T2:MutDistribution<f64>+Clone {
    pub fn new 
    (traffic_source: Box<Queue>, 
        file_logger: Box<Queue>, 
        _n_servers: usize, 
        _link_distribution: T1, 
        _server_distribution: T2, 
        _lb_policy: CentralizedLBPolicy
    ) -> Self {
        let n = 0;
        let mut _qn = QNet::new();
        let _ptraffic_source = _qn.add_queue(traffic_source);
        let _pfile_logger = _qn.add_queue(file_logger);
        let mut ret = CentralizedAutoscalingQNet {
            qn : _qn,
            n_servers: _n_servers,
            ptraffic_source: _ptraffic_source,
            pfile_logger: _pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n],
            lb_policy: _lb_policy,
            link_distribution: _link_distribution.clone(),
            server_distribution: _server_distribution.clone()
        };
        for _i in 0..ret.n_servers {
            ret.add_server();
        }
        ret
    }


    pub fn make_transition (&mut self) -> f64
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
                        let _dest_q = f(&r, &self.qn);
                        r.add_log_entry(t, (orig_q, _dest_q));
                        //println!("Transition: {}->{}", orig_q, _dest_q);
                        self.qn.queues[_dest_q].arrival(r);

                    }
                }

            }
        }
        next_exit
    }

    fn do_autoscale(&mut self) 
    {
        //TODO fill me!
    }
    fn update_network(&mut self)
    {
        let last_server_idx = self.n_servers - 1;

        if self.n_servers >= 1 {
            // Transition src -> link(src, server $rand) 
            {
                let source = self.ptraffic_source;
                let dests = self.pnetwork_arcs.clone();  
                let servers = self.pservers.clone();
                let n_servers = self.n_servers;

                //NB: the following removes stale transitions as well, since we use a new version of pnetwork_arcs
                match self.lb_policy {
                    CentralizedLBPolicy::RND => 
                        self.qn.add_transition(source, Box::new(move |ref _req, ref _qn| { 
                            dests[rand::thread_rng().gen_range(0, n_servers)]
                        })),
                    CentralizedLBPolicy::JSQ2 => 
                        self.qn.add_transition(source, Box::new(move |ref _req, ref qn| { 
                            let choice_1 = servers[rand::thread_rng().gen_range(0, n_servers)];
                            let choice_2 = servers[rand::thread_rng().gen_range(0, n_servers)];
                            let load_1 = qn.get_queue(choice_1).read_load();
                            let load_2 = qn.get_queue(choice_2).read_load();
                            if load_1 < load_2 {
                                dests[choice_1]
                            } else {
                                dests[choice_2]
                            }

                        })),
                    CentralizedLBPolicy::JIQ =>
                        self.qn.add_transition(source, Box::new(move |ref _req, ref qn| { 
                            // Join an idle queue...
                            for i in 0..n_servers {
                                if qn.get_queue(i).read_load() == 0 {
                                    return i
                                }
                            }
                            // ... or go to a random one, if none is available
                            dests[rand::thread_rng().gen_range(0, n_servers)]
                        }))                                          
                }
                
            }

            // Transition link(src, server n) -> server(n)
            {
                let source = self.pnetwork_arcs[last_server_idx];
                let dest = self.pservers[last_server_idx];
                self.qn.add_transition(source, Box::new(move |ref _req, ref _qn| dest ));
            }


            // Transition server n -> file_logger
            {
                let source = self.pservers[last_server_idx];
                let dest = self.pfile_logger;
                self.qn.add_transition(source, Box::new(move |_,_| dest ));
            }

        } else {
            // If there are no servers, sink the traffic source to /dev/null
            let dest = self.pfile_logger;
            self.qn.add_transition(self.ptraffic_source,  Box::new(move |_,_| dest ));
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
            // Link src -> server n
            self.pnetwork_arcs.push(self.qn.add_queue(Box::new(MGINF::new(1., self.link_distribution.clone()))));
            // Server n
            self.pservers.push(self.qn.add_queue(Box::new(MG1PS::new(1., self.server_distribution.clone()))));

            if self.pservers.len() != self.n_servers {
                panic!("|pserver| != n_servers after adding server!");
            }
        } else {
            /* Reuse slot in vector if already existing */
            // Link src -> server n
            self.pnetwork_arcs[self.n_servers - 1] = self.qn.change_queue(self.pnetwork_arcs[self.n_servers - 1], Box::new(MGINF::new(1., self.link_distribution.clone())));
            // Server n
            self.pservers[self.n_servers - 1] = self.qn.change_queue(self.pservers[self.n_servers - 1], Box::new(MG1PS::new(1., self.server_distribution.clone())));
        }

        self.do_autoscale();
        self.update_network();   

        //println!("n_servers = {}", self.n_servers);
    }

    fn remove_server(&mut self) {
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
