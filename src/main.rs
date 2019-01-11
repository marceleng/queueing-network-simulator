#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
pub mod float_binaryheap;
pub mod distribution;

extern crate rand;
extern crate zipf;

use queues::poisson_generator::PoissonGenerator;

//use std::env;
//use std::rc::Rc;
//use std::cell::RefCell;

use rand::distributions::{Exp};
use distribution::{ConstantDistribution};

use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use queues::queueing_network::QNet;
use queues::file_logger::FileLogger;


fn run_sim() {

    let n_servers = 5;

    let mu = 1./0.100; //100 ms
    let lambda = 3. * mu;
    let tau_network = 0.000_200; //200 μs

    // the servers
    let mut pservers = vec![0 as usize; n_servers];   
    // the links between the servers (to emulate network RTT)
    let mut pnetwork_arcs = vec![0 as usize; n_servers];


    let mut qn = QNet::new();
    let traffic_source = qn.add_queue(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))));

    for i in 0..n_servers {
        // Link (server i-1 [or source] -> server i)
        pnetwork_arcs[i] = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_network))));       
        // Server i
        pservers[i] = qn.add_queue(Box::new(MG1PS::new(1., Exp::new(mu))));        
    }

    let file_logger = qn.add_queue(Box::new(FileLogger::new(1024, "test.csv")));


    // Transitions src -> link(server -1 [==src], server 0)
    let pnetwork_arc0 = pnetwork_arcs[0];
    qn.add_transition(traffic_source, Box::new(move |_,_| pnetwork_arc0));

    // Transitions link(server i-1, server i) -> { server(i) or link(server i, server i+1) }
    for i in 0..n_servers {
        let source = pnetwork_arcs[i];
        let potential_dest = pservers[i];
        let fallback_dest = if i < (n_servers-1) { pnetwork_arcs[i+1] } else { potential_dest };
        qn.add_transition(source, Box::new(move |ref _req, ref qn| { 
            let load = qn.read_queue(potential_dest).read_load();
            println!("load={}", load);
            if load == 0 {
                potential_dest
            } else {
                fallback_dest
            }
        }));
    }

    // Transitions server(i) -> file logger
    for i in 0..n_servers {
        let source = pservers[i];
        qn.add_transition(source, Box::new(move |_, _| file_logger ));
    }



    // Run simulation
    for _ in 0..100_000 {
        qn.make_transition();
    }
    println!("Done");

}



fn main () {   
    run_sim()
}
