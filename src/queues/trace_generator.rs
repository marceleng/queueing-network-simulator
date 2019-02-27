use std::io::BufReader;
use std::io::BufRead;
use std::fs::File;

use helpers::float_binaryheap::FloatBinaryHeap;
use queues::request::Request;
use queues::Queue;

pub struct TraceGenerator {
    requests: FloatBinaryHeap<usize>,
    next_exit: f64,
    next_request: Option<Request>,
}

impl TraceGenerator {
    pub fn new(csv_filename: &'static str, csv_delimiter: char) -> Self {
        let mut ret = TraceGenerator {
            requests: FloatBinaryHeap::new(),
            next_exit: 0.,
            next_request: None
        };

        let sched_csv = File::open(csv_filename).unwrap();
        let buf_read = BufReader::new(sched_csv);

        for line in buf_read.lines() {
            let l = line.unwrap();
            let mut s = l.split(csv_delimiter);
            if let Some(s1) = s.next() {
                if let Ok(t) = s1.parse() {
                    if let Some(s2) = s.next() {
                        if let Ok(work) = s2.parse() {
                            ret.requests.push(t, work);
                        } else {
                            ret.requests.push(t, 0);
                        }
                    } else {
                        ret.requests.push(t, 0);
                    }
                }
            }
        }

        ret.pop_next_exit();
        ret        
    }

    fn generate_next_exit(&mut self) {
        if let Some((t, work)) = self.requests.pop() {
            self.next_exit = t;
            self.next_request = Some(Request::new(work));
        } else {
            self.next_request = None;
        }
    }
}

impl Queue for TraceGenerator {
    fn arrival (&mut self, _req: Request) {
        panic!("You should not arrive at a generator");
    }

    fn update_time (&mut self, _time: f64) {}

    fn read_next_exit (&self) -> Option<(f64,&Request)> {
        match self.next_request {
            None => None,
            Some(ref r) => Some((self.next_exit,r))
        }
    }

    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> {
        let ret = (self.next_exit, self.next_request.take());
        self.generate_next_exit();
        match ret.1 {
            None => None,
            Some(r) => Some((ret.0,r))
        }
    }

    fn read_load (&self) -> usize { 1 }        
}
