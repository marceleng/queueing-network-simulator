use std::vec::Vec;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use queues::queueing_network::{QNet,Transition,TransitionError};
use queues::Queue;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;

use queues::request::Request;

use distribution::MutDistribution;

use float_binaryheap::FloatBinaryHeap;
use rand::Rng;

pub enum CentralizedLBPolicy {
    RND,
    JSQ2,
    JIQ
}

struct ScalingSchedule(FloatBinaryHeap<usize>, Option<(f64,Request)>);

impl ScalingSchedule {

    pub fn from_csv (filename: &'static str, delimiter: char) -> Self {
        let mut ret = ScalingSchedule {
            0: FloatBinaryHeap::new(),
            1: None
        };

        let sched_csv = File::open(filename).unwrap();
        let buf_read = BufReader::new(sched_csv);

        for line in buf_read.lines() {
            let l = line.unwrap();
            let mut s = l.split(delimiter);
            let t = s.next().unwrap().parse().unwrap();
            let n = s.next().unwrap().parse().unwrap();
            ret.0.push(t,n);
        }

        // Let's initialize ret.1
        ret.pop_next_exit();
        ret
    }
}

impl Queue for ScalingSchedule {
    fn arrival        (&mut self, _: Request) { panic!("You should not arrive in this queue"); }

    fn update_time    (&mut self, _time: f64) {}

    fn read_next_exit (&self) -> Option<(f64,&Request)> {
        match self.1 {
            None=>None,
            Some((t, ref r)) => Some((t,r))
        }
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        let ret = self.1.take();
        self.1 = self.0.pop().map( |(t,n)| (t, Request::new(n)));
        ret
    }

    fn read_load	  (&self) -> usize { 1 }
}

pub struct CentralizedAutoscalingQNet<T1: 'static+ MutDistribution<f64>+Clone,T2: 'static+ MutDistribution<f64>+Clone> {
    qn: QNet,
    n_servers: usize,
    ptraffic_source: usize,
    pfile_logger: usize,
    pservers: Vec<usize>,
    pnetwork_arcs: Vec<usize>,
    lb_policy: CentralizedLBPolicy,
    scaling_queue: usize,
    link_distribution: T1,
    server_distribution: T2
}

impl<T1,T2> CentralizedAutoscalingQNet<T1,T2> where T1:MutDistribution<f64>+Clone, T2:MutDistribution<f64>+Clone {
    pub fn new (traffic_source: Box<Queue>,
                file_logger: Box<Queue>,
                _n_servers: usize,
                //schedule_csv: &'static str,
                _link_distribution: T1,
                _server_distribution: T2,
                _lb_policy: CentralizedLBPolicy) -> Self {
        let n = 0;
        let mut _qn = QNet::new();
        let _ptraffic_source = _qn.add_queue(traffic_source);
        let _pfile_logger = _qn.add_queue(file_logger);
        let mut ret = CentralizedAutoscalingQNet {
            qn : _qn,
            n_servers: 0,
            ptraffic_source: _ptraffic_source,
            pfile_logger: _pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n],
            lb_policy: _lb_policy,
            scaling_queue: std::usize::MAX,
            link_distribution: _link_distribution.clone(),
            server_distribution: _server_distribution.clone()
        };
        for _i in 0.._n_servers {
            ret.add_server();
        }
        ret
    }


    pub fn make_transition (&mut self) -> Result<Transition, TransitionError>
    {
        let ret = self.qn.make_transition();

        //Let's catch the transitions from the scaling queue
        if let Err(TransitionError::DestinationOutOfBound(t)) = ret {
            if t.origin == self.scaling_queue {
                let goal = t.request;
                while self.n_servers > goal {
                    self.remove_server();
                }
                while self.n_servers < goal {
                    self.add_server();
                }

                Ok(t)
            }
            else {
                Err(TransitionError::DestinationOutOfBound(t))
            }
        }
        else {
            ret
        }
            
    }

    fn setup_autoscale(&mut self)
    {
        //TODO: pass the file as an argument or in struct members
        self.scaling_queue = self.qn.add_queue(Box::new(ScalingSchedule::from_csv("schedule.csv", ' ')));
        //std::usize::MAX makes sure that we raise an Error in make_transition()
        self.qn.add_transition(self.scaling_queue, Box::new(|_,_| std::usize::MAX));
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

        self.update_network();

        //println!("n_servers = {}", self.n_servers);
    }

    fn remove_server(&mut self) {
        // We don't actually perform removal of the queues, so that they can flush naturally
        // However, we update the network so that no transition points to those queues

        if self.n_servers > 0 {
            self.n_servers -= 1;

            self.update_network();
        }
        //println!("n_servers = {}", self.n_servers);

    }
}
