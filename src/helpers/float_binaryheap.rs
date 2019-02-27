extern crate ordered_float;
extern crate num_traits;

use self::ordered_float::{NotNan,FloatIsNan};
use std::collections::BinaryHeap;
use std::cmp::Ordering;
use self::num_traits::cast::ToPrimitive;

#[derive(PartialEq)]
struct HeapEntry<T> where T: PartialEq {
    pub key: NotNan<f64>,
    pub value: T
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

impl<T> HeapEntry<T> where T: PartialEq {
    pub fn to_tuple(&self) -> (f64, &T) {
        (self.key.to_f64().unwrap(), &self.value)
    }

    pub fn from_tuple (key : f64, value : T) -> Self {
        let key = NotNan::new(key);
        let key = match key {
            Ok(num) => num,
            Err(FloatIsNan) => {
                panic!("Float is Nan in Heap")
            }
        };
        HeapEntry {
            key,
            value
        }
    }
}

#[derive(Default)]
pub struct FloatBinaryHeap<T> where T: PartialEq {
    heap: BinaryHeap<HeapEntry<T>>,
}

impl<T> FloatBinaryHeap<T> where T: PartialEq {
    pub fn new () -> FloatBinaryHeap<T> {
        FloatBinaryHeap {
            heap: BinaryHeap::new()
        }
    }

    pub fn push (&mut self, key: f64, value: T) {
        self.heap.push(HeapEntry::from_tuple(key,value))
    }

    pub fn peek(&self) -> Option<(f64, &T)> {
        match self.heap.peek() {
            None => None,
            Some(r) => Some(r.to_tuple())
        }
    }

    pub fn pop (&mut self) -> Option<(f64, T)> {
        match self.heap.pop() {
            None => None,
            Some(r) => {
                Some((r.key.to_f64().unwrap(), r.value))
            }
        }
    }

    pub fn len (&self) -> usize {
        self.heap.len()
    }

    pub fn is_empty(&self) -> bool {
        self.heap.is_empty()
    }

    pub fn translate_keys(&mut self, translation :f64) {
        let translation = match NotNan::new(translation) {
            Ok(num) => num,
            Err(FloatIsNan) => panic!("Translation value is Nan")
        };
        let items : Vec<HeapEntry<T>> = self.heap.drain().collect();
        for mut item in items {
            item.key += translation;
            self.heap.push(item)
        }
    }
}
