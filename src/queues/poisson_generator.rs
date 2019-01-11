extern crate rand;

use self::rand::distributions::Exp;
use self::rand::distributions::Distribution;
use queues::request::Request;
use queues::Queue;

pub struct PoissonGenerator<T> where T: Distribution<usize> {
    next_exit: f64,
    next_request: Option<Request>,
    pop_distribution: T,
    iat_distribution: Exp
}

impl<T> Queue for PoissonGenerator<T> where T: Distribution<usize> {
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

    fn read_load (&self) -> usize {
        1
    }    
}

impl<T> PoissonGenerator<T> where T: Distribution<usize> {
    pub fn new (rate: f64, distribution: T) -> Self{
        let mut ret = PoissonGenerator {
            next_exit: 0.,
            next_request: None,
            pop_distribution: distribution,
            iat_distribution: Exp::new(rate)
        };
        ret.generate_next_exit ();
        ret
    }

    fn generate_next_exit(&mut self) {
        self.next_exit += self.iat_distribution.sample(&mut rand::thread_rng());
        self.next_request = Some(Request::new(self.pop_distribution.sample(&mut rand::thread_rng())));
    }
}
