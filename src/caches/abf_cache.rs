extern crate bloomfilter;
use caches::Cache;

use std::mem::swap;

use self::bloomfilter::Bloom;

pub struct AgingBloomFilter {
    na: usize,
    size_a1: usize,
    a1: Bloom,
    a2: Bloom,
}

impl AgingBloomFilter {
    pub fn new(nb_elements: usize, miss_rate: f64) -> Self {
        let fa = 1. - (1. - miss_rate).sqrt();
        AgingBloomFilter {
            na: nb_elements,
            size_a1: 0,
            a1: Bloom::new_for_fp_rate(nb_elements, fa),
            a2: Bloom::new_for_fp_rate(nb_elements, fa),
        }
    }

    fn invert_and_clear(&mut self) {
        swap(&mut self.a2, &mut self.a1);
        self.a1.clear();
        self.size_a1 = 0;
    }

    pub fn check_and_set(&mut self, item: usize) -> bool {
        let ret;
        if self.a1.check(item) {
            ret = true;
        }
        else {
            ret = self.a2.check(item);
            self.a1.set(item);
            self.size_a1 += 1;
            if self.size_a1 == self.na {
                self.invert_and_clear();
            }
        }
        ret
    }

    pub fn get_active_size (&self) -> usize {
        self.size_a1
    }

    pub fn resize(&mut self, new_na: usize) {
        if new_na < self.na {
            if self.size_a1 > new_na {
                self.invert_and_clear();
            }
            else {
                self.a2.clear();
            }
        }
        self.na = new_na
    }
}

impl Cache<usize> for AgingBloomFilter {
    fn contains (&mut self, entry: &usize) -> bool {
        self.a1.check(entry) || self.a2.check(entry)
    }

    fn update (&mut self, entry: usize) {
        if !self.a1.check(entry) {
            self.a1.set(entry);
            self.size_a1 += 1;
            if self.size_a1 == self.na {
                swap(&mut self.a2, &mut self.a1);
                self.a1.clear();
                self.size_a1 = 0;
            }
        }
    }
}
