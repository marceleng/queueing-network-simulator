#![allow(dead_code)]
#![warn(unused_imports)]

mod queues;
mod caches;
mod zipf;
pub mod float_binaryheap;
pub mod llist;

extern crate rand;

use rand::distributions::Exp;

use queues::mg1ps::MG1PS;
use queues::zipfgen::ZipfGenerator;
use queues::queueing_network::QNet;
use queues::request::Request;
use caches::lru_cache::LruCache;
use caches::Cache;
use caches::lru_cache::Iter;

fn main() {
    let mut cache: LruCache<u64> = LruCache::new(5);

    println!("{}",cache.contains(&3));
    cache.update(5);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(4);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(5);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(3);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(2);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(1);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    cache.update(0);
    for iter in cache.iter() {
        print!("{} ", iter)
    }
    println!("");
    println!("{}", cache);
}
