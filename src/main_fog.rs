#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
pub mod float_binaryheap;
pub mod distribution;

extern crate rand;
extern crate zipf;

use zipf::ZipfDistribution;
use rand::distributions::Distribution;

use std::env;
use std::rc::Rc;
use std::cell::RefCell;

use rand::distributions::{Exp};
use distribution::{ConstantDistribution,OffsetExp};

use queues::mg1ps::MG1PS;
use queues::mginf::MGINF;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::{QNet,TransitionError};
use queues::file_logger::FileLogger;


use caches::lru_cache::LruCache;
use caches::abf_fpga_cache::AgingBloomFilterFPGA;
use caches::Cache;

fn run_sim() {

    let catalogue_size = 10_000_000;
    let alpha = 1.01;

    let x_comp = 1.;
    let s_raw = 1e6;
    let s_proc = 1e4;
    //let delta_app = 100. * 1e-3;

    //let s_cachef_bytes = 1e9;
    let s_cachef_bytes = 1e5;
    let c_compf = 3. * 1e2;
    let c_acc = (10. / 8.) * 1e9;
    let tau_acc = 4. * 1e-3;
    let tau_tlsf = tau_acc;

    let c_compc = 2e2;
    let tau_db = 1e-3;
    let c_core = (1./8.) * (1e9);
    let tau_core = 40. * 1e-3;
    let tau_tlsc = tau_core + tau_acc;

    let s_cachec = 3.1e5;
    //let phi_opt = 0.42;

    //let k_LFU_2s = 1.2e6;
    //let k_LFU = 6.1e5;
    //let k_lru = 1.3e6;
    let k_lru = 2.1e5;
    let na = 149_850;
    let k1 = 387_707;

    let s_cachef = s_cachef_bytes/s_proc;

    let lambda = 10000.;

    let nb_arrivals = 100_000_000;
    let logfile = "result.csv";

    //let mut filter = P2LruFilter::new(5*k_lru as usize, delta_app-tau_acc, percentile);
    //filter.set_optimize(true);
    //let filter = P2LruFilter::new(10, delta_app-tau_acc, percentile);
    let filter = AgingBloomFilterFPGA::new(k1,0.01);
    //let mut filter: LruCache<usize> = LruCache::new(k_lru as usize);
    let filter_ptr = Rc::new(RefCell::new(filter));
    let fog_cache: LruCache<usize> = LruCache::new(s_cachef as usize);
    let fcache_ptr = Rc::new(RefCell::new(fog_cache));
    let cloud_cache: LruCache<usize> = LruCache::new(s_cachec as usize);
    let ccache_ptr = Rc::new(RefCell::new(cloud_cache));

    let mut qn = QNet::new();
    //let source = qn.add_queue(Box::new(
    //        ZipfGenerator::new(alpha, catalogue_size, |x| Exp::new(x*lambda))));
    let source = qn.add_queue(Box::new(
            ZipfGenerator::new(alpha, catalogue_size, Exp::new(lambda), nb_arrivals)));

    let fog_proc = qn.add_queue(Box::new(MG1PS::new(c_compf, Exp::new(x_comp))));
    let tls_acc_d = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));
    let tls_acc_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));

    let core_d = qn.add_queue(Box::new(MG1PS::new(c_core, OffsetExp::new(tau_core, s_proc))));
    let cloud_proc = qn.add_queue(Box::new(MGINF::new(c_compc, Exp::new(x_comp))));
    let acc_u = qn.add_queue(Box::new(MG1PS::new(c_acc, OffsetExp::new(tau_acc, s_raw))));
    let acc_d  = qn.add_queue(Box::new(MG1PS::new(c_acc, OffsetExp::new(tau_acc, s_proc))));
    let tls_core_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsc))));
    let db_queue = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_db))));


    let log = qn.add_queue(Box::new(FileLogger::new(1024, logfile)));

    let filter_clone = filter_ptr.clone();
    qn.add_transition(source, Box::new(move |req,_| {
        let mut cache = filter_clone.borrow_mut();
        //let content = (req.get_id(), req.get_content());
        let content = req.get_content();
        let ret =
            if cache.contains(&content) { tls_acc_u }
            else { tls_core_u };
        cache.update(content);
        ret
    }));

    //FOG
    let fcache_clone = fcache_ptr.clone();
    qn.add_transition(tls_acc_u, Box::new(move |req,_| {
        let mut cache = fcache_clone.borrow_mut();
        if cache.contains(&req.get_content()) {
            cache.update(req.get_content());
            acc_d
        }
        else {
            tls_acc_d
        }
    }));
    qn.add_transition(tls_acc_d, Box::new(move |_,_| acc_u));
    qn.add_transition(acc_u, Box::new(move |_,_| fog_proc));
    let filter_clone = filter_ptr.clone();
    qn.add_transition(fog_proc, Box::new(move |req,_| {
        fcache_ptr.borrow_mut().update(req.get_content());
        //filter_clone.borrow_mut().update((req.get_id(), req.get_content()));
        filter_clone.borrow_mut().update(req.get_content());
        acc_d
    }));

    //CLOUD
    let ccache_clone = ccache_ptr.clone();
    qn.add_transition(tls_core_u, Box::new(move |req,_| {
        let mut cache = ccache_clone.borrow_mut();
        if cache.contains(&req.get_content()) {
            cache.update(req.get_content());
            core_d
        }
        else {
            db_queue
        }
    }));
    qn.add_transition(db_queue, Box::new(move | _,_ | cloud_proc));
    qn.add_transition(cloud_proc, Box::new(move |req,_| {
        ccache_ptr.borrow_mut().update(req.get_content());
        core_d
    }));

    //let filter_ptr_1 = filter_ptr.clone();
    qn.add_transition(core_d, Box::new(move |_req,_| {
        //filter_ptr_1.borrow_mut().update((req.get_id(), req.get_content()));
        //filter_ptr_1.borrow_mut().update(req.get_content());
        //println!("{}", filter_ptr_1.borrow());
        acc_d
    }));

    qn.add_transition(acc_d, Box::new(move |_,_| log));

    //qn.add_queue(Box::new(P2LruFilterCont::new(filter_ptr)));
    let mut res = qn.make_transition();
    while res.is_ok() {
        res = qn.make_transition();
    }
    match res.unwrap_err() {
        TransitionError::NoExitFound => println!("Done"),
        _ => panic!("Unexpected error")
    };
}


fn sixcn_sim(catalogue_size: usize, alpha: f64, cache_size: usize, filter_size: usize ) {
     let mut cache: LruCache<usize> = LruCache::new(cache_size);
     let mut filter: LruCache<usize> = LruCache::new(filter_size);


     let source = ZipfDistribution::new(catalogue_size, alpha).unwrap();

     let nb_iterations = catalogue_size*10;
     let mut hit_rate = 0.;

     for _ in 0..nb_iterations {
        let next_content = source.sample(&mut rand::thread_rng());
        if filter.contains(&next_content) {
            if cache.contains(&next_content) {
                hit_rate += 1. / (nb_iterations as f64);
            }
            cache.update(next_content);
        }
        filter.update(next_content);
     }

     println!("{},{}", filter_size, hit_rate);
}


fn main () {
   
    /*
    let args: Vec<String> = env::args().collect();

    let catalogue_size:usize = args[1].parse::<usize>().unwrap();
    let alpha:f64 = args[2].parse::<f64>().unwrap();
    let cache_delta: f64 = args[3].parse::<f64>().unwrap();
    //let filter_delta: f64 = args[4].parse::<f64>().unwrap();

    let cache_size = (catalogue_size as f64 * cache_delta) as usize;

    for filter_delta_int in 1..10 {
        let filter_size = catalogue_size * filter_delta_int / 100;
        sixcn_sim(catalogue_size, alpha, cache_size, filter_size);
    }
    */
    
    run_sim()
}
