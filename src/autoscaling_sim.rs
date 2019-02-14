#![allow(dead_code)]
#![warn(unused_imports)]

use std::env;
use std::f64::consts::PI;

extern crate rand;
extern crate zipf;

use queues::poisson_generator::PoissonGenerator;
use queues::cm_poisson_generator::ContinuouslyModulatedPoissonGenerator;


use rand::distributions::{Exp};
use distribution::{ConstantDistribution};

use queues::autoscaling_qnetwork::AutoscalingQNet;
use queues::centralized_autoscaling_qnetwork::CentralizedAutoscalingQNet;
use queues::centralized_autoscaling_qnetwork::CentralizedLBPolicy;
use queues::file_logger::FileLogger;


fn centralized_noautoscaling_sim(n_servers: usize, rho: f64)
{
    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_000; //200 μs

    let mut qn = CentralizedAutoscalingQNet::new(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, &format!("results/results_rnd_{:.2}.csv", rho))),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu),
                                      CentralizedLBPolicy::RND);


    // Run simulation
    let mut t = 0.;
    while t < 400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}

fn centralized_autoscaling_sim(n_servers: usize)
{
    let mu = 1./0.100; //100 ms
    let tau_network = 0.000_000; //200 μs

    let mut qn = CentralizedAutoscalingQNet::new(Box::new(ContinuouslyModulatedPoissonGenerator::new(
                                                                Box::new(move |t| mu*(50. - 20.*(2.*PI*t/86400.).cos())),
                                                          ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, "results/results_centralized_autoscale.csv")),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu),
                                      CentralizedLBPolicy::RND);


    // Run simulation
    let mut t = 0.;
    while t < 86400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}

fn sr_noautoscaling_sim(n_servers: usize, rho: f64)
{
    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_000; //200 μs

    let mut qn = AutoscalingQNet::new(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, &format!("results/results_sr_{:.2}.csv", rho))),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu));


    // Run simulation
    let mut t = 0.;
    while t < 400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}

fn sr_autoscaling_sim(n_servers: usize)
{
    let mu = 1./0.100; //100 ms
    let tau_network = 0.000_000; //200 μs

    let mut qn = AutoscalingQNet::new(Box::new(ContinuouslyModulatedPoissonGenerator::new(
                                                                Box::new(move |t| mu*(50. - 20.*(2.*PI*t/86400.).cos())),
                                                          ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, "results/results_sr_autoscale.csv")),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu));


    // Run simulation
    let mut t = 0.;
    while t < 86400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}



pub fn run_autoscaling (_: env::Args) {
    let mut rho = 4.;
    while rho <= 40. {
        centralized_noautoscaling_sim(40, rho);
        rho += 5.;
    }

}
