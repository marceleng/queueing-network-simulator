#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
mod fog_cloud_sim;
pub mod float_binaryheap;
pub mod distribution;

use std::env;

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


fn autoscaling_sim(rho: f64) {

    let n_servers = 60;

    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_200; //200 μs


    let mut qn = AutoscalingQNet::new(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))), 
                                      Box::new(FileLogger::new(1024, &format!("results/results_{:.2}.csv", rho))));

    for _i in 0..n_servers {
        qn.add_server(ConstantDistribution::new(tau_network), /*Exp::new(mu)*/ConstantDistribution::new(1./mu));
    }
    //qn.add_server(ConstantDistribution::new(tau_network), Exp::new(mu));
    //qn.remove_server();

    // Run simulation
    for _ in 0..10_000_000 {
        qn.make_transition();
    }
    println!("Done");

}



fn run_autoscaling (_: env::Args) {
    let mut rho = 43.6;
    while rho <= 43.6 {
        autoscaling_sim(rho);
        rho += 0.1;
    }
}

fn main() {
    let mut args = env::args();
    args.next();
    let exp = args.next().expect("No experiment provided as runtime argument");
    if exp == "autoscaling" {
        run_autoscaling(args);
    }
    else if exp == "fog" {
        fog_cloud_sim::run(args);
    }
    else {
        panic!("Could not recognize experiment: {}", exp);
    }
}
