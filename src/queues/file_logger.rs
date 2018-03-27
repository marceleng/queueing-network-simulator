use std::vec::Vec;
use std::fs::File;
use std::io::prelude::*;
use std::io::Result;

use queues::request::Request;
use queues::Queue;


pub struct FileLogger {
    buffer: Vec<Request>,
    buffer_size: usize,
    file: File,
}

impl FileLogger {

    pub fn new (buffer_size: usize, filename: &str) -> Self
    {
        FileLogger {
            buffer: Vec::with_capacity(buffer_size),
            buffer_size,
            file: File::create(filename).expect(&("Could not open file ".to_owned() + filename))
        }
    }

    fn dump_log (&mut self) -> Result<()>
    {
        let s: Vec<String> = self.buffer.drain(..).map(|req: Request| { 
            let log_str: Vec<String> = req.get_log().into_iter().map(|(key,(orig,dest))| {
                format!("{}:{}:{}", key, orig, dest)
            }).collect();
            let log_str: String = log_str.join(";");
            format!("{},{},{}", req.get_id(), req.get_content(), log_str)
        }).collect();
        self.file.write_all(s.join("\n").as_bytes())
    }
}

impl Queue for FileLogger {

    fn arrival(&mut self, req: Request)
    {
        self.buffer.push(req);
        if self.buffer.len() >= self.buffer_size {
            self.dump_log().expect("Failed to write log");
        }
    }

    fn update_time(&mut self, _: f64) {}

    fn read_next_exit(&self) -> Option<(f64, &Request)> {None}

    fn pop_next_exit(&mut self) -> Option<(f64,Request)> {None}
}

impl Drop for FileLogger {

    fn drop(&mut self) {
        self.dump_log().expect("Failed to write log on drop");
    }
}
