use caches::Cache;

use std::iter::Iterator;
use std::hash::Hash;
use std::cmp::Eq;

use std::collections::HashMap;

use std::ptr;
use std::fmt::{Display,Formatter,Result};

struct LruNode<T> {
    elem: T,
    child: Link<T>,
    //TODO: Change to Option<std::ptr::NonNull<LruNode<T>>>
    parent: *mut LruNode<T>,
}

type Link<T> = Option<Box<LruNode<T>>>;

impl<T> LruNode<T> {
    pub fn new(elem: T) -> Self {
        LruNode { elem, child: None, parent: ptr::null_mut() }
    }

    pub fn insert_after(&mut self, elem: T) {
        let mut new_node = Box::new(LruNode::new(elem));
        let old_child = self.child.take();
        let raw_self: *mut _ = &mut *self;
        let raw_node: *mut _ = &mut *new_node;

        new_node.parent = raw_self;
        if let Some(mut old_child) = old_child {
            old_child.parent = raw_node;
            new_node.child = Some(old_child);
        }
        self.child = Some(new_node);
    }
}

pub struct LruCache<T> where T: Hash+Eq+Copy {
    lru_size: usize,
    nb_objects: usize,
    head: Link<T>,
    tail: *mut LruNode<T>,
    nodes: HashMap<T, *mut LruNode<T>>,
}

pub struct IntoIter<T>(LruCache<T>) where T: Hash+Eq+Copy;

pub struct Iter<'a, T:'a> where T: Hash+Eq+Copy {
    next: Option<&'a LruNode<T>>,
}

pub struct IterMut<'a, T:'a> where T: Hash+Eq+Copy {
    next: Option<&'a mut LruNode<T>>,
}

impl<T> LruCache<T> where T: Hash+Eq+Copy {

    pub fn new (lru_size: usize) -> Self {
        LruCache {
            lru_size,
            nb_objects: 0,
            head: None,
            tail: ptr::null_mut(),
            nodes: HashMap::new()
        }
    }

    unsafe fn rm_node(&mut self, node: *mut LruNode<T>)
    {
        let node = node.as_mut().expect("You should not call rm_node with null_ptr");
        let mut child = node.child.take();
        match child {
            Some(ref mut child) => { child.parent = node.parent; }
            None => { self.tail = node.parent; }
        };
        if !(*node).parent.is_null() {
                (*node.parent).child = child;
        }
        else {
            self.head = child;
        }
        self.nb_objects -=1;
    }

    fn pop_tail(&mut self) -> Option<T> {
           if !self.tail.is_null() {
               self.nb_objects -= 1;
               let old_tail = self.tail;
               unsafe {
                   self.nodes.remove(&(*old_tail).elem);
                   self.tail = (*old_tail).parent;
                   let ret = (*old_tail).elem;
                   match self.tail.as_mut() {
                       Some(new_tail) => new_tail.child.take(),
                       None => self.head.take()
                   };
                   Some(ret)
               }
           }
           else { None }
    }

    fn push_head(&mut self, mut new_node: Box<LruNode<T>>) {
        self.nodes.insert(new_node.elem, &mut *new_node);
        if self.tail.is_null() {
            self.tail = &mut *new_node;
        }
        if let Some(ref mut old_head) = self.head {
            old_head.parent = &mut *new_node;
        }
        new_node.child = self.head.take();
        self.head = Some(new_node);
        self.nb_objects += 1;
    }

    fn rm_node_if_exists(&mut self, entry: T) {
        if let Some(node_ptr) = self.nodes.remove(&entry) {
            unsafe {
                self.rm_node(node_ptr);
            }
        }
    }

    pub fn resize(&mut self, new_size: usize) {
        self.lru_size = new_size;
        let diff = self.nb_objects - self.lru_size;
        if diff > 0 {
            self.pop_n_nodes(diff);
        }
    }

    fn resize_and_return (&mut self, new_size : usize) -> Option<Box<LruNode<T>>> {
        self.lru_size = new_size;
        let diff = self.nb_objects - self.lru_size;
        if diff > 0 {
            self.pop_n_nodes(diff)
        }
        else {
            None
        }
    }

    fn pop_n_nodes (&mut self, n: usize) -> Option<Box<LruNode<T>>> {

        assert!(n <= self.nb_objects, "Tried to remove too many objects from Lru Cache");

        if n > 0 {
            let mut n = n;
            self.nb_objects -= n;

            unsafe {
                let mut cur_node = self.tail;
                while n > 0 {
                    n -= 1;
                    cur_node = (*cur_node).parent;
                }
                if (*cur_node).parent.is_null() { //It's the head
                    self.tail = ptr::null_mut();
                    self.head.take()
                }
                else {
                    let parent = (*cur_node).parent;
                    self.tail = parent;
                    (*cur_node).parent = ptr::null_mut();
                    (*parent).child.take()
                }
            }
        } else {
            None
        }
    }

    pub fn into_iter(self) -> IntoIter<T>
    {
        IntoIter(self)
    }

    pub fn iter(&self) -> Iter<T>
    {
        unsafe{
            Iter { next: self.tail.as_ref() }
        }
    }

    pub fn iter_mut(&mut self) -> IterMut<T>
    {
        unsafe {
            IterMut { next: self.tail.as_mut() }
        }
    }

}

impl<T> Display for LruCache<T> where T: Hash+Eq+Copy+Display {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut it = self.iter();
        if let Some(fst) = it.next() {
            try!(write!(f, "{}", fst));
            for item in it {
                try!(write!(f, ",{}", item));
            }
        }
        Ok(())
    }
}


impl<T> Cache<T> for LruCache<T> where T: Hash+Eq+Copy {

    fn contains (&self, entry: &T) -> bool {
        match self.nodes.get(entry) {
            None => false,
            Some(&node) => {
                if node.is_null() {
                    false
                }
                else {
                    true
                }
            }
        }
    }

    fn update (&mut self, entry: T) {
        self.rm_node_if_exists(entry);
        let node = Box::new(LruNode::new(entry));
        self.push_head(node);
        if self.nb_objects > self.lru_size {
            self.pop_tail();
        }
    }
}

impl<T> Drop for LruCache<T> where T: Hash+Eq+Copy {
    fn drop(&mut self) {
        let mut cur_node = self.head.take();
        while let Some(mut boxed_node) = cur_node {
            cur_node = boxed_node.child.take();
        }
    }
}

type Pit<IdType: Hash+Eq> = HashMap<IdType, f64>;

pub struct PitLruFilter<ContentType, IdType, OptFunc> where
    IdType: Hash+Eq,
    OptFunc: Fn(&Pit<IdType>) -> usize,
    ContentType: Hash+Eq+Copy
{
    filter_limit: usize,
    pit: Pit<IdType>,
    accept: LruCache<ContentType>,
    refuse: LruCache<ContentType>,
    opt_func: OptFunc
}


impl<ContentType, IdType, OptFunc> PitLruFilter<ContentType, IdType, OptFunc> where
    IdType: Hash+Eq,
    OptFunc: Fn(&Pit<IdType>) -> usize,
    ContentType: Hash+Eq+Copy
{
    pub fn new (filter_max_size: usize, opt_func: OptFunc) -> Self {
        PitLruFilter {
            filter_limit: filter_max_size,
            pit: Pit::new(),
            accept: LruCache::new(filter_max_size),
            refuse: LruCache::new(0),
            opt_func
        }
    }

    fn recompute_filter_pos (&mut self) {
        let diff = (self.opt_func)(&self.pit);
    }
}



impl<T> Iterator for IntoIter<T> where T: Hash+Eq+Copy {
    type Item = T;
    fn next (&mut self) -> Option<Self::Item> {
        self.0.pop_tail()
    }
}

impl<'a, T> Iterator for Iter<'a, T> where T: Hash+Eq+Copy {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.next.map(|node| {
                self.next = node.parent.as_ref();
                &node.elem
            })
        }
    }
}

impl<'a, T> Iterator for IterMut<'a, T> where T: Hash+Eq+Copy {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        unsafe {
            self.next.take().map(|node| {
                self.next = node.parent.as_mut();
                &node.elem
            })
        }
    }
}
