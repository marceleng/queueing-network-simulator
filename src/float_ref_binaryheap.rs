extern crate ordered_float;
extern crate num_traits;

use self::ordered_float::NotNaN;
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use std::result::Result;
use self::num_traits::cast::ToPrimitive;

#[derive(PartialEq)]
struct HeapEntry<T> where T: PartialEq {
    key: NotNaN<f64>,
    value: T
}

impl<T> Eq for HeapEntry<T> where T: PartialEq {}

impl<T> PartialOrd for HeapEntry<T> where T: PartialEq {
    fn partial_cmp(&self, other: &HeapEntry<T>) -> Option<Ordering> {
        Some(other.key.cmp(&self.key))
    }
}

impl<T> Ord for HeapEntry<T> where T: PartialEq {
    fn cmp(&self, other:&HeapEntry<T>) -> Ordering {
        other.key.cmp(&self.key)
    }
}

pub struct FloatBinaryHeap< T> where T: PartialEq {
    heap: BinaryHeap<HeapEntry<T>>,
}

impl<T> FloatBinaryHeap< T> where T: PartialEq {
    pub fn new () -> FloatBinaryHeap<T> {
        FloatBinaryHeap {
            heap: BinaryHeap::new()
        }
    }

    pub fn insert (&mut self, key: f64, value: T) {
        let key = NotNaN::new(key);
        let key = match key {
            Ok(num) => num,
            Err(error) => {
                panic!("Float is NaN in Heap")
            }
        };
        let entry = HeapEntry {
            key,
            value
        };
        self.heap.push(entry)
    }

    pub fn peek(&self) -> Option<(f64, &T)> {
        match self.heap.peek() {
            None => None,
            Some(r) => Some((r.key.to_f64().unwrap(), &r.value))
        }
    }

}
