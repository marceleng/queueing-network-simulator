#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
mod zipf;
pub mod float_binaryheap;
pub mod llist;
pub mod distribution;

extern crate rand;

use std::rc::Rc;
use std::cell::RefCell;

use rand::distributions::Exp;
use distribution::ConstantDistribution;

use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::QNet;
use queues::file_logger::FileLogger;

use queues::request::Request;

use caches::lru_cache::LruCache;
use caches::Cache;

fn main() {

    let catalogue_size = 10000000;
    let alpha = 1.01;

    let x_comp = 1e7;
    let s_raw = 1e6;
    let s_proc = 1e4;
    let delta_app = 100. * 1e-3;

    let s_cachef_B = 1e9;
    let c_compf = 3. * 1e9;
    let c_acc = (10. / 8.) * 1e9;
    let tau_acc = 4. * 1e-3;
    let tau_TLSf = tau_acc;

    let c_compc = 2e9;
    let tau_db = 1e-3;
    let c_core = (1./8.) * (1e9);
    let tau_core = 40. * 1e-3;
    let tau_TLSc = tau_core + tau_acc;

    let s_cachec = 3.1e5;
    let phi_opt = 0.42;

    let k_LFU_2s = 1.2e6;
    let k_LFU = 6.1e5;
    let k_LRU = 1.3e6;

    let s_cachef = s_cachef_B/s_proc;

    let lambda = 2000.;

    let filter = LruCache::new(k_LRU as usize);
    let fog_cache = LruCache::new(s_cachef as usize);
    let cloud_cache = LruCache::new(s_cachec as usize);

    let mut qn = QNet::new();
    let source = qn.add_queue(Box::new(
            ZipfGenerator::new(alpha, catalogue_size, |x| Exp::new(x*lambda))));

    let fog_proc = qn.add_queue(Box::new(MG1PS::new(c_compf, Exp::new(x_comp))));
    let tls_acc_d = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_TLSf))));
    let tls_acc_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_TLSf))));

    let core_d = qn.add_queue(Box::new(MG1PS::new(c_core, Exp::new(s_proc))));
    let cloud_proc = qn.add_queue(Box::new(MGINF::new(c_compc, Exp::new(x_comp))));


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
