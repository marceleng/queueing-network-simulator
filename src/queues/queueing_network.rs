use queues::Queue;
use std::vec::Vec;

pub struct QNet {
    queues: Vec<Box<Queue>>,
}
