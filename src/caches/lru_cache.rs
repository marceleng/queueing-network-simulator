extern crate lru_cache;

use caches::Cache;

use std::hash::Hash;
use std::cmp::Eq;

pub type LruCache<T> = self::lru_cache::LruCache<T,i32>;

impl<T> Cache<T> for LruCache<T> where T: Hash+Eq+Copy {

    fn contains (&mut self, entry: &T) -> bool {
        self.contains_key(entry)
    }

    fn update (&mut self, entry: T) {
        self.insert(entry, 0);
    }
}

/*
impl<T,V> Drop for LruCache<T,V> where T: Hash+Eq+Copy {
    fn drop(&mut self) {
        let mut cur_node = self.head.take();
        while let Some(mut boxed_node) = cur_node {
            cur_node = boxed_node.child.take();
        }
    }
}*/
