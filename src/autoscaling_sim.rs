#![allow(dead_code)]
#![warn(unused_imports)]

use std::env;
use std::f64::consts::PI;

extern crate rand;
extern crate zipf;

use queues::poisson_generator::PoissonGenerator;
use queues::cm_poisson_generator::ContinuouslyModulatedPoissonGenerator;


use rand::distributions::{Exp};
use helpers::distribution::{ConstantDistribution};

use queues::autoscaling_qnetwork::AutoscalingQNet;
use queues::autoscaling_qnetwork::AutoscalingParameters;
use queues::centralized_autoscaling_qnetwork::CentralizedLoadBalancingQNet;
use queues::centralized_autoscaling_qnetwork::CentralizedLBPolicy;
use queues::centralized_autoscaling_qnetwork::CentralizedScalingPolicy;
use queues::file_logger::FileLogger;
use queues::trace_generator::TraceGenerator;

fn centralized_lb_noautoscaling_sim(n_servers: usize, rho: f64)
{
    let mu = 1./0.100; //100 ms
    let lambda = rho * mu;
    let tau_network = 0.000_000; //200 μs

    let mut qn = CentralizedLoadBalancingQNet::new(Box::new(PoissonGenerator::new(lambda, ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, &format!("results/results_rnd_{:.2}.csv", rho))),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu),
                                      CentralizedLBPolicy::RND,
                                      CentralizedScalingPolicy::NoAutoscaling);


    // Run simulation
    let mut t = 0.;
    while t < 400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}

fn centralized_lb_with_schedule_sim(n_servers: usize)
{
    let mu = 1./0.100; //100 ms
    let tau_network = 0.000_000; //200 μs

    let mut qn = CentralizedLoadBalancingQNet::new(Box::new(ContinuouslyModulatedPoissonGenerator::new(
                                                                Box::new(move |t| mu*(50. - 20.*(2.*PI*t/86400.).cos())),
                                                          ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, "results/results_centralized_autoscale.csv")),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu),
                                      CentralizedLBPolicy::RND,
                                      CentralizedScalingPolicy::Schedule("schedule.csv", ' ') );


    // Run simulation
    let mut t = 0.;
    while t < 86400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}



fn centralized_lb_with_autoscaling_sim(n_servers: usize)
{
    let mu = 1./0.100; //100 ms
    let tau_network = 0.000_000; //200 μs

    let mut qn = CentralizedLoadBalancingQNet::new(Box::new(ContinuouslyModulatedPoissonGenerator::new(
                                                                Box::new(move |t| mu*(50. - 20.*(2.*PI*t/86400.).cos())),
                                                          ConstantDistribution::new(1))),
                                      Box::new(FileLogger::new(1024, "results/results_centralized_autoscale.csv")),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      Exp::new(mu),
                                      CentralizedLBPolicy::RND,
                                      CentralizedScalingPolicy::Autoscaling(0.110, 0.104, 60.) );


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
                                      Exp::new(mu),
                                      None);


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
                                      Exp::new(mu),
                                      Some(AutoscalingParameters{proba_empty: 0.8, ewma_window_len: 100.}));


    // Run simulation
    let mut t = 0.;
    while t < 86400. {
        t = qn.make_transition().unwrap().time;
    }
    println!("Done");

}

fn sr_autoscaling_sim_with_trace(n_servers: usize)
{
    let mu = 1./0.100; //100 ms
    let tau_network = 0.000_000; //200 μs

    let mut qn = AutoscalingQNet::new(Box::new(TraceGenerator::new("trace.csv", ' ')),
                                      Box::new(FileLogger::new(1024, "results/results_sr_autoscale_trace.csv")),
                                      n_servers,
                                      ConstantDistribution::new(tau_network),
                                      ConstantDistribution::new(1./mu),
                                      Some(AutoscalingParameters{proba_empty: 0.8, ewma_window_len: 100.}));


    // Run simulation
    while let Ok(_) = qn.make_transition() {}

    println!("Done");

}



pub fn run_autoscaling (_: env::Args) {
    sr_autoscaling_sim_with_trace(40);
}
