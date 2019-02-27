use std::vec::Vec;
use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use queues::queueing_network::{QNet,Transition,TransitionError};
use queues::Queue;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use queues::file_logger::FileLogger;

use queues::request::Request;

use helpers::distribution::MutDistribution;

use helpers::float_binaryheap::FloatBinaryHeap;
use helpers::ewma::TimeWindowedEwma;

use rand::Rng;

pub enum CentralizedLBPolicy {
    RND,
    JSQ2,
    JIQ
}

pub enum CentralizedScalingPolicy {
    Schedule(&'static str, char),
    Autoscaling(f64,f64,f64),
    NoAutoscaling,
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
    fn arrival (&mut self, _: Request) { panic!("You should not arrive in this queue"); }

    fn update_time (&mut self, _time: f64) {}

    fn read_next_exit (&self) -> Option<(f64,&Request)> {
        match self.1 {
            None=>None,
            Some((t, ref r)) => Some((t,r))
        }
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)> {
        let ret = self.1.take();
        self.1 = self.0.pop().map( |(t,n)| (t, Request::new(n)));
        ret
    }

    fn read_load (&self) -> usize { 1 }
}

struct AutoscalingFileLogger {
    nb_servers: usize,
    upscale_threshold: f64,
    downscale_threshold: f64,
    num_events: usize,
    num_events_to_converge: usize,
    log: FileLogger,
    ewma: TimeWindowedEwma,
    exit: Option<Request>,
    time: f64,
}

impl AutoscalingFileLogger {
    pub fn new (buffer_size: usize, filename: &str, initial_number_of_servers: usize,
                upscale_threshold: f64, downscale_threshold: f64, ewma_window_len: f64)
                -> Self
    {
        AutoscalingFileLogger {
            nb_servers: initial_number_of_servers,
            upscale_threshold,
            downscale_threshold,
            log: FileLogger::new(buffer_size, filename),
            ewma: TimeWindowedEwma::new(ewma_window_len),
            exit: None,
            time: 0.,
            num_events: 0,
            num_events_to_converge: 100
        }
    }

    pub fn from_file_logger (log: FileLogger, initial_number_of_servers: usize,
                             upscale_threshold: f64, downscale_threshold: f64, ewma_window_len: f64) -> Self
    {
        AutoscalingFileLogger {
            nb_servers: initial_number_of_servers,
            upscale_threshold,
            downscale_threshold,
            log,
            ewma: TimeWindowedEwma::new(ewma_window_len),
            exit: None,
            time: 0.,
            num_events: 0,
            num_events_to_converge: 100
        }
    }

}

impl Queue for AutoscalingFileLogger
{

    fn arrival (&mut self, r: Request)
    {
        let service_time = self.ewma.update(self.time, r.get_current_lifetime());

        if service_time > self.upscale_threshold &&
            self.num_events >= self.num_events_to_converge  && self.nb_servers < std::usize::MAX {
            self.num_events = 0;
            self.nb_servers += 1;
            self.exit = Some(Request::new(self.nb_servers))
        }
        else if service_time < self.downscale_threshold &&
            self.num_events >= self.num_events_to_converge && self.nb_servers > 0 {
            self.num_events = 0;
            self.nb_servers -= 1;
            self.exit = Some(Request::new(self.nb_servers))
        }

        self.num_events += 1;

        self.log.arrival(r);
    }

    fn update_time (&mut self, time: f64)
    {
        self.log.update_time(time);
        self.time = time;
    }

    fn read_next_exit (&self) -> Option<(f64,&Request)>
    {
        let t = self.time;
        self.exit.as_ref().map(|x| { (t, x) })
    }

    fn pop_next_exit (&mut self) -> Option<(f64,Request)>
    {
        let ret = self.exit.take();
        ret.map(|x| { (self.time, x) })
    }

    fn read_load (&self) -> usize
    {
        match self.exit {
            Some(_) => 1,
            None => 0
        }
    }
}

pub struct CentralizedLoadBalancingQNet<T1: 'static+ MutDistribution<f64>+Clone,T2: 'static+ MutDistribution<f64>+Clone> {
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

impl<T1,T2> CentralizedLoadBalancingQNet<T1,T2> where T1:MutDistribution<f64>+Clone, T2:MutDistribution<f64>+Clone {
    pub fn new (traffic_source: Box<Queue>,
                file_logger: Box<FileLogger>,
                n_servers: usize,
                link_distribution: T1,
                server_distribution: T2,
                lb_policy: CentralizedLBPolicy,
                autoscaling_policy: CentralizedScalingPolicy) -> Self {
        let n = 0;
        let mut qn = QNet::new();
        let ptraffic_source = qn.add_queue(traffic_source);

        let pfile_logger = match autoscaling_policy {
            CentralizedScalingPolicy::Autoscaling(up, down, wlen) =>
                qn.add_queue(
                    Box::new(
                        AutoscalingFileLogger::from_file_logger(*file_logger, n_servers,
                                                                up, down, wlen))),
            _ => qn.add_queue(file_logger)
        };

        let scaling_queue = match autoscaling_policy {
            CentralizedScalingPolicy::Autoscaling(_,_,_) => pfile_logger,
            CentralizedScalingPolicy::Schedule(filename, delimiter) =>
                qn.add_queue(Box::new(ScalingSchedule::from_csv(filename, delimiter))),
            _ => std::usize::MAX
        };

        if scaling_queue != std::usize::MAX {
            //std::usize::MAX makes sure that we raise an Error in make_transition()
            qn.add_transition(scaling_queue, Box::new(|_,_| std::usize::MAX));
        }

        let mut ret = CentralizedLoadBalancingQNet {
            qn,
            n_servers: 0,
            ptraffic_source,
            pfile_logger,
            pservers: vec![0 as usize; n],
            pnetwork_arcs: vec![0 as usize; n],
            lb_policy,
            scaling_queue,
            link_distribution: link_distribution.clone(),
            server_distribution: server_distribution.clone()
        };
        for _ in 0..n_servers {
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
                            let choice_1 = rand::thread_rng().gen_range(0, n_servers);
                            let mut choice_2 = choice_1;
                            while choice_2 == choice_1 && n_servers > 1 {
                                choice_2 = rand::thread_rng().gen_range(0, n_servers);
                            }
                            let load_1 = qn.get_queue(servers[choice_1]).read_load();
                            let load_2 = qn.get_queue(servers[choice_2]).read_load();
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
                                if qn.get_queue(servers[i]).read_load() == 0 {
                                    return dests[i]
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
            self.qn.change_queue(self.pnetwork_arcs[self.n_servers - 1], Box::new(MGINF::new(1., self.link_distribution.clone())));
            // Server n
            self.qn.change_queue(self.pservers[self.n_servers - 1], Box::new(MG1PS::new(1., self.server_distribution.clone())));
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
