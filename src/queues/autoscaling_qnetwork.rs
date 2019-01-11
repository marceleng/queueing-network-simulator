use queues::queueing_network::QNet;
use queues::Queue;
use queues::request::Request;
use std::vec::Vec;
use std::f64::INFINITY;
use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use distribution::MutDistribution;


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



    pub fn add_server<T1: 'static+ MutDistribution<f64>,T2: 'static+ MutDistribution<f64>>(&mut self, link_distribution: T1, server_distribution: T2)
    {
        self.n_servers += 1;

        // Link { server n-1 [or src] } -> server n
        self.pnetwork_arcs.push(self.qn.add_queue(Box::new(MGINF::new(1., link_distribution))));
        // Server n
        self.pservers.push(self.qn.add_queue(Box::new(MG1PS::new(1., server_distribution))));
        
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
            panic!("n_servers == 0 after adding server!");
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
}