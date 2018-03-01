static mut REQUEST_COUNTER: u64 = 0;

#[derive(PartialEq,Clone,Debug)]
pub struct Request {
    id: u64,
    content: u64,
    log: Vec<(f64,String)>,
}

impl Request {
    pub fn new (content: u64) -> Self {
        let mut ret = Request {
            id: 0,
            content,
            log : Vec::new()
        };
        unsafe {
            ret.id = REQUEST_COUNTER;
            REQUEST_COUNTER += 1;
        }
        ret
    }
}
