mod queues;
mod zipf;
pub mod float_binaryheap;

extern crate rand;

use rand::distributions::{Exp,Range};

use queues::request::Request;
use queues::mg1ps::MG1PS;
use queues::zipfgen::ZipfGenerator;
use queues::Queue;
use zipf::Zipf;

fn main() {
    let mut q = MG1PS::new(2.0, Exp::new(1.0));
    let mut p = ZipfGenerator::new(1.0, 1000, |x| Exp::new(x*1.0));
    for i in 0..100 {
        let req = match p.pop_next_exit() {
            None => panic!("Should not happen"),
            Some(r) => r
        };
        println!("Arrival: {:?}",req);
    }
}
