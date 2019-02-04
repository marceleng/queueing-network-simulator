#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
mod fog_cloud_sim;
mod autoscaling_sim;
pub mod float_binaryheap;
pub mod distribution;

use std::env;

extern crate rand;
extern crate zipf;



fn main() {
    let mut args = env::args();
    args.next();
    let exp = args.next().expect("No experiment provided as runtime argument");
    if exp == "autoscaling" {
        autoscaling_sim::run_autoscaling(args);
    }
    else if exp == "fog" {
        fog_cloud_sim::run(args);
    }
    else {
        panic!("Could not recognize experiment: {}", exp);
    }
}
