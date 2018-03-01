mod queues;
mod zipf;
pub mod float_binaryheap;

extern crate rand;

use rand::distributions::{Exp,Range};

use queues::request::Request;
use queues::mg1ps::MG1PS;
use queues::poisson_generator::PoissonGenerator;
use queues::Queue;
use zipf::Zipf;

fn main() {
    let mut q = MG1PS::new(2.0, Exp::new(1.0));
    //TODO: turn it on its head: it should be a ZipfGenerator that takes Exp as an argument
    let mut p = PoissonGenerator::new(1.0, Zipf::new(1.0, 1000));
    for i in 0..100 {
        let req = match p.pop_next_exit() {
            None => panic!("Should not happen"),
            Some(r) => r
        };
        println!("Arrival: {:?}",req);
    }
}
