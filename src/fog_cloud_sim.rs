#![allow(dead_code)]
#![warn(unused_imports)]

extern crate rand;
extern crate zipf;

use zipf::ZipfDistribution;
use rand::distributions::Distribution;

use std::rc::Rc;
use std::cell::RefCell;

use std::env;

use rand::distributions::{Exp};
use distribution::ConstantDistribution;

use queues::mg1ps::{AggregatingMG1PS,MG1PS};
use queues::mginf::MGINF;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::{QNet,TransitionError};
use queues::file_logger::FileLogger;


use caches::lru_cache::LruCache;
use caches::abf_fpga_cache::AgingBloomFilterFPGA;
use caches::Cache;
use caches::{PerfectLfu,RandomAccept};

enum Filter {
    Blind(f64),
    Lru(usize),
    Lfu(usize),
    Abf(usize),
}

fn run_sim(mode: Filter, s_cachec: usize) {

    let catalogue_size = 10_000_000;
    let alpha = 1.0;

    let x_comp = 1e7;
    let s_raw = 1e6;
    let s_proc = 1e4;

    let s_cachef_bytes = 1e9;
    let c_compf = 3e9;
    let c_acc = (10. / 8.) * 1e9;
    let tau_acc = 4e-3;
    let tau_tlsf = tau_acc;

    let c_compc = 2e9;
    let tau_db = 1e-3;
    let c_core = (1./8.) * (1e9);
    let tau_core = 40. * 1e-3;
    let tau_tlsc = tau_core + tau_acc;

    let s_cachef = s_cachef_bytes/s_proc;

    let lambda = 10000.;

    let nb_arrivals = 500_000_000;
    let logfile = match mode {
        Filter::Lru(_) => "result-lru.csv",
        Filter::Abf(_) => "result-abf.csv",
        Filter::Blind(_) => "result-blind.csv",
        Filter::Lfu(_) => "result-lfu.csv",
    };

    let filter_ptr: Rc<RefCell<Cache<usize>>> = match mode {
        Filter::Lru(klru) => Rc::new(RefCell::new(LruCache::new(klru))),
        Filter::Abf(k1) => Rc::new(RefCell::new(AgingBloomFilterFPGA::new(k1,0.01))),
        Filter::Blind(phi) => Rc::new(RefCell::new(RandomAccept::from_value(phi).unwrap())),
        Filter::Lfu(klfu) => Rc::new(RefCell::new(PerfectLfu::new(klfu))),
    };

    //let filter: LruCache<usize> = LruCache::new(k_lru as usize);
    //let filter_ptr = Rc::new(RefCell::new(filter));
    let fog_cache: LruCache<usize> = LruCache::new(s_cachef as usize);
    let fcache_ptr = Rc::new(RefCell::new(fog_cache));
    let cloud_cache: LruCache<usize> = LruCache::new(s_cachec as usize);
    let ccache_ptr = Rc::new(RefCell::new(cloud_cache));

    let mut qn = QNet::new();
    let source = qn.add_queue(Box::new(
            ZipfGenerator::new(alpha, catalogue_size, Exp::new(lambda), nb_arrivals)));

    let fog_proc = qn.add_queue(Box::new(AggregatingMG1PS::new(c_compf, Exp::new(1. / x_comp))));
    let tls_acc_d = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));
    let tls_acc_u = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_tlsf))));

    let core_d = qn.add_queue(Box::new(MG1PS::new(c_core, Exp::new(1. / s_proc))));
    let core_d_propagation = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_core))));

    let cloud_proc = qn.add_queue(Box::new(MGINF::new(c_compc, Exp::new(1. / x_comp))));

    let acc_u = qn.add_queue(Box::new(AggregatingMG1PS::new(c_acc, Exp::new(1. / s_raw))));
    let acc_u_propagation = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_acc))));

    let acc_d  = qn.add_queue(Box::new(MG1PS::new(c_acc, Exp::new(1. / s_proc))));
    let acc_d_propagation = qn.add_queue(Box::new(MGINF::new(1., ConstantDistribution::new(tau_acc))));

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
    qn.add_transition(acc_u, Box::new(move |_,_| acc_u_propagation));
    qn.add_transition(acc_u_propagation, Box::new(move |_,_| fog_proc));

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
    qn.add_transition(core_d, Box::new(move |_,_| core_d_propagation));
    qn.add_transition(core_d_propagation, Box::new(move |_,_| acc_d ));

    qn.add_transition(acc_d, Box::new(move |_,_| acc_d_propagation));
    qn.add_transition(acc_d_propagation, Box::new(move |_,_| log));

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


pub fn run (mut args: env::Args) {
   
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
    let filter_arg = args.next().expect("No filter specified at runtime");
    let mode = match filter_arg.as_ref() {
        "abf" => Filter::Abf(392_787),
        "lru" => Filter::Lru(212_292),
        "blind" => Filter::Blind(0.084),
        "lfu" => Filter::Lfu(145_456),
        _ => panic!("Unrecognized filter: {}", filter_arg)
    };
    let s_cachec = match mode {
        Filter::Abf(_) | Filter::Lru(_) => 1_995_569,
        Filter::Lfu(_) => 2_000_000,
        Filter::Blind(_) => 3_100_000,
    };

    run_sim(mode, s_cachec);
}
