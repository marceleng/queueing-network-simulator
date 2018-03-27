use std::collections::VecDeque;

use queues::Queue;
use queues::request::Request;

pub struct PassthroughQueue {
    time: f64,
    requests: VecDeque<Request>,
}

impl Queue for PassthroughQueue {
    fn arrival (&mut self, req: Request)
    {
        self.requests.push_back(req);
    }

    fn update_time (&mut self, time: f64)
    {
        self.time = time;
    }

    fn read_next_exit (&self) -> Option<(f64, &Request)>
    {
        self.requests.front().map(|r| (self.time, r))
    }

    fn pop_next_exit (&mut self) -> Option<(f64, Request)>
    {
        self.requests.pop_front().map(|r| (self.time, r))
    }
}
