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
use distribution::{ConstantDistribution,OffsetExp};

use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::QNet;
use queues::file_logger::FileLogger;


use caches::lru_cache::LruCache;
use caches::Cache;

fn main() {

    let catalogue_size = 10000000;
    let alpha = 1.01;

    let x_comp = 1e7;
    let s_raw = 1e6;
    let s_proc = 1e4;
    //let delta_app = 100. * 1e-3;

    let s_cachef_bytes = 1e9;
    let c_compf = 3. * 1e9;
    let c_acc = (10. / 8.) * 1e9;
    let tau_acc = 4. * 1e-3;
    let tau_tlsf = tau_acc;

    let c_compc = 2e9;
    let tau_db = 1e-3;
    let c_core = (1./8.) * (1e9);
    let tau_core = 40. * 1e-3;
    let tau_tlsc = tau_core + tau_acc;

    let s_cachec = 3.1e5;
    //let phi_opt = 0.42;

    //let k_LFU_2s = 1.2e6;
    //let k_LFU = 6.1e5;
    let k_lru = 1.3e6;

    let s_cachef = s_cachef_bytes/s_proc;

    let lambda = 2000.;

    let filter = LruCache::new(k_lru as usize);
    let filter_ptr = Rc::new(RefCell::new(filter));
    let fog_cache: LruCache<usize> = LruCache::new(s_cachef as usize);
    let fcache_ptr = Rc::new(RefCell::new(fog_cache));
    let cloud_cache: LruCache<usize> = LruCache::new(s_cachec as usize);
    let ccache_ptr = Rc::new(RefCell::new(cloud_cache));

    let mut qn = QNet::new();
    let source = qn.add_queue(Box::new(
            ZipfGenerator::new(alpha, catalogue_size, |x| Exp::new(x*lambda))));

    let fog_proc = qn.add_queue(Box::new(MG1PS::new(c_compf, Exp::new(x_comp))));
    let tls_acc_d = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));
    let tls_acc_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));

    let core_d = qn.add_queue(Box::new(MG1PS::new(c_core, OffsetExp::new(tau_core, s_proc))));
    let cloud_proc = qn.add_queue(Box::new(MGINF::new(c_compc, Exp::new(x_comp))));
    let acc_u = qn.add_queue(Box::new(MG1PS::new(c_acc, OffsetExp::new(tau_acc, s_raw))));
    let acc_d  = qn.add_queue(Box::new(MG1PS::new(c_acc, OffsetExp::new(tau_acc, s_proc))));
    let tls_core_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsc))));
    let db_queue = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_db))));


    let log = qn.add_queue(Box::new(FileLogger::new(1000, "test.csv")));

    let filter_clone = filter_ptr.clone();
    qn.add_transition(source, Box::new(move |req| {
        let ret = if filter_clone.borrow().contains(&req.get_content()) {
            filter_clone.borrow_mut().update(req.get_content());
            tls_acc_u } else { tls_core_u };
        ret
    }));

    //FOG
    let fcache_clone = fcache_ptr.clone();
    qn.add_transition(tls_acc_u, Box::new(move |req| {
        if fcache_clone.borrow().contains(&req.get_content()) {
            fcache_clone.borrow_mut().update(req.get_content());
            acc_d
        }
        else {
            tls_acc_d
        }
    }));
    qn.add_transition(tls_acc_d, Box::new(move |_| acc_u));
    qn.add_transition(acc_u, Box::new(move |_| fog_proc));
    let filter_clone = filter_ptr.clone();
    qn.add_transition(fog_proc, Box::new(move |req| {
        fcache_ptr.borrow_mut().update(req.get_content());
        filter_clone.borrow_mut().update(req.get_content());
        acc_d
    }));

    //CLOUD
    let ccache_clone = ccache_ptr.clone();
    qn.add_transition(tls_core_u, Box::new(move |req| {
        if ccache_clone.borrow().contains(&req.get_content()) {
            ccache_clone.borrow_mut().update(req.get_content());
            core_d
        }
        else {
            db_queue
        }
    }));
    qn.add_transition(db_queue, Box::new(move | _ | cloud_proc));
    qn.add_transition(cloud_proc, Box::new(move |req| {
        ccache_ptr.borrow_mut().update(req.get_content());
        core_d
    }));
    qn.add_transition(core_d, Box::new(move |req| {
        filter_ptr.borrow_mut().update(req.get_content());
        acc_d
    }));

    qn.add_transition(acc_d, Box::new(move |_| log));

    for _ in 0..400000000 {
        qn.make_transition();
    }
}
