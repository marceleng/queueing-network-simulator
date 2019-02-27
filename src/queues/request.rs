static mut REQUEST_COUNTER: usize = 0;

type LogKey   = f64;
type LogEntry = (usize, usize);

#[derive(PartialEq,Clone,Debug)]
pub struct Request(Box<_Request>);

#[derive(PartialEq,Clone,Debug)]
struct _Request {
    id: usize,
    content: usize,
    log: Vec<(LogKey, LogEntry)>,
}

impl Request {
    pub fn new (content: usize) -> Self {
        Request { 0: Box::new(_Request::new(content)) }
    }

    pub fn get_content (&self) -> usize {
        self.0.get_content()
    }

    pub fn get_id(&self) -> usize {
        self.0.get_id()
    }

    pub fn add_log_entry(&mut self, key: LogKey, entry: LogEntry)
    {
        self.0.log.push((key, entry));
    }

    pub fn get_log(&self) -> Vec<(LogKey, LogEntry)>
    {
        self.0.log.clone()
    }

    pub fn get_current_lifetime(&self) -> f64
    {
        if self.0.log.len() <= 1 {
            0.
        }
        else {
            self.0.log.last().unwrap().0 - self.0.log.first().unwrap().0
        }
    }
}

impl _Request {
    pub fn new (content: usize) -> Self {
        let mut ret = _Request {
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

    pub fn get_content (&self) -> usize {
        self.content
    }

    pub fn get_id(&self) -> usize {
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
