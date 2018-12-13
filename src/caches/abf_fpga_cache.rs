extern crate bloomfilter;
use caches::Cache;

use std::mem::swap;

use self::bloomfilter::Bloom;

pub struct AgingBloomFilterFPGA {
    k1: usize,
    arrival_cnt: usize,
    a1: Bloom,
    a2: Bloom,
}

impl AgingBloomFilterFPGA {
    pub fn new(k1: usize, miss_rate: f64) -> Self {
        let fa = 1. - (1. - miss_rate).sqrt();
        AgingBloomFilterFPGA {
            k1,
            arrival_cnt: 0,
            a1: Bloom::new_for_fp_rate(k1, fa),
            a2: Bloom::new_for_fp_rate(k1, fa),
        }
    }

    fn invert_and_clear(&mut self) {
        swap(&mut self.a2, &mut self.a1);
        self.a1.clear();
        self.arrival_cnt = 0;
    }

    pub fn check_and_set(&mut self, item: usize) -> bool {
        let ret = self.contains(&item);
        self.update(item);

        ret
    }

    pub fn get_arrival_cnt (&self) -> usize {
        self.arrival_cnt
    }
}

impl Cache<usize> for AgingBloomFilterFPGA {
    fn contains (&mut self, entry: &usize) -> bool {
        self.a1.check(entry) || self.a2.check(entry)
    }

    fn update (&mut self, entry: usize) {
        self.a1.set(entry);
        self.arrival_cnt += 1;
        if self.arrival_cnt == self.k1 {
            self.invert_and_clear();
        }
    }
}
