#![allow(dead_code)]
#![warn(unused_imports)]

use std::env;

extern crate rand;
extern crate zipf;

use queues::poisson_generator::PoissonGenerator;
use queues::cm_poisson_generator::ContinuouslyModulatedPoissonGenerator;


use rand::distributions::{Exp};
use distribution::{ConstantDistribution};

use queues::autoscaling_qnetwork::AutoscalingQNet;
use queues::file_logger::FileLogger;


fn autoscaling_sim(rho: f64) {

    let n_servers = 40;

    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_000; //200 μs

    let mut qn = AutoscalingQNet::new(Box::new(ContinuouslyModulatedPoissonGenerator::new(Box::new(move |t| mu*(50. - 20.*(2.*3.14159265*t/86400.).cos())), ConstantDistribution::new(1))), 
                                      //Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))), 
                                      Box::new(FileLogger::new(1024, &format!("results/results_{:.2}.csv", rho))));

    for _i in 0..n_servers {
        qn.add_server(ConstantDistribution::new(tau_network), /*Exp::new(mu)*/ConstantDistribution::new(1./mu));
    }
    //qn.add_server(ConstantDistribution::new(tau_network), Exp::new(mu));
    //qn.remove_server();

    // Run simulation
    let mut t = 0.;
    while t < 86400. {
        t = qn.make_transition();
    }
    println!("Done");

}



pub fn run_autoscaling (_: env::Args) {
    let mut rho = 43.6;
    while rho <= 43.6 {
        autoscaling_sim(rho);
        rho += 0.1;
    }
}
