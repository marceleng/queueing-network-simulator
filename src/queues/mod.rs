pub mod request;
pub mod mg1ps;
pub mod mginf;
pub mod mgkfifo;
pub mod poisson_generator;
pub mod zipfgen;
pub mod queueing_network;
pub mod file_logger;
pub mod passthrough;

use self::request::Request;

#[derive(Clone)]
pub struct Process {
    req: Request,
    work: f64
}

pub trait Queue {
    fn arrival        (&mut self, Request);
    fn update_time    (&mut self, time: f64);
    fn read_next_exit (&self) -> Option<(f64,&Request)>;
    fn pop_next_exit  (&mut self) -> Option<(f64,Request)>;
}
