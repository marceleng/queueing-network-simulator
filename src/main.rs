#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
mod zipf;
pub mod float_binaryheap;
pub mod llist;

extern crate rand;

use std::rc::Rc;
use std::cell::RefCell;

use rand::distributions::Exp;

use queues::mg1ps::MG1PS;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::QNet;
use queues::file_logger::FileLogger;

use queues::request::Request;

use caches::lru_cache::LruCache;
use caches::Cache;

fn main() {

    let mut qn = QNet::new();
    let gen_idx = qn.add_queue(Box::new(ZipfGenerator::new(1.0, 100000, |x| Exp::new(x*1.0))));
    let comp_idx = qn.add_queue(Box::new(MG1PS::new(2.0, Exp::new(1.0))));
    let log_idx = qn.add_queue(Box::new(FileLogger::new(1000, "test.csv")));

    let cache = LruCache::new(1000);
    let cache = Rc::new(RefCell::new(cache));
    let cache_copy = cache.clone();

    qn.add_transition(gen_idx, Box::new(move | r: &Request | {
        if cache.borrow().contains (&r.get_content()) {
            cache.borrow_mut().update(r.get_content());
            log_idx
        }
        else {
            comp_idx
        }
    }));
    qn.add_transition(comp_idx, Box::new(move | r: &Request | {
        cache_copy.borrow_mut().update(r.get_content());
        log_idx
    }));

    for _ in 0..100000 {
        qn.make_transition();
    }
}
