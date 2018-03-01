pub mod request;
pub mod mg1ps;
pub mod poisson_generator;
pub mod zipfgen;
pub mod queueing_network;

use self::request::Request;

pub trait Queue {
    fn arrival        (&mut self, Request);
    fn update_time    (&mut self, time: f64);
    fn read_next_exit (&self) -> Option<(f64,&Request)>;
    fn pop_next_exit  (&mut self) -> Option<(f64,Request)>;
}
