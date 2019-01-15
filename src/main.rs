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

use queues::autoscaling_qnetwork::AutoscalingQNet;
use queues::file_logger::FileLogger;


fn run_sim(rho: f64) {

    let n_servers = 5;

    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_200; //200 μs


    let mut qn = AutoscalingQNet::new(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))), 
                                      Box::new(FileLogger::new(1024, &format!("results/results_{:.2}.csv", rho))));

    for _i in 0..n_servers {
        qn.add_server(ConstantDistribution::new(tau_network), Exp::new(mu));
    }
    qn.add_server(ConstantDistribution::new(tau_network), Exp::new(mu));
    qn.remove_server();

    // Run simulation
    for _ in 0..500_000 {
        qn.make_transition();
    }
    println!("Done");

}



fn main () {   
    let mut rho = 0.1;
    while rho <= 4.0 {
        run_sim(rho);
        rho += 0.1;
    }
}
