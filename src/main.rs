mod queues;
mod zipf;
pub mod float_binaryheap;

extern crate rand;

use rand::distributions::Exp;

use queues::mg1ps::MG1PS;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::QNet;
use queues::request::Request;

fn main() {
    let mut qn = QNet::new();
    let q1: usize = qn.add_queue(Box::new(MG1PS::new(2.0, Exp::new(1.0))));
    let q2 = qn.add_queue(Box::new(ZipfGenerator::new(5.0, 1000, |x| Exp::new(x*1.0))));

    let trans = move |_ :&Request| q1;

    println!("{}", q1);

    qn.add_transition(q2, Box::new(trans));
    for _ in 0..1000 {
        qn.make_transition()
    }
}
