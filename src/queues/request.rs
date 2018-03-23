static mut REQUEST_COUNTER: u64 = 0;

type LogKey   = f64;
type LogEntry = (usize, usize);

#[derive(PartialEq,Clone,Debug)]
pub struct Request {
    id: u64,
    content: u64,
    log: Vec<(LogKey, LogEntry)>,
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

    pub fn get_content (&self) -> u64 {
        self.content
    }

    pub fn get_id(&self) -> u64 {
        self.id
    }

    pub fn add_log_entry(&mut self, key: LogKey, entry: LogEntry)
    {
        self.log.push((key, entry));
    }

    pub fn get_log(&self) -> Vec<(LogKey, LogEntry)>
    {
        self.log.clone()
    }
}
