use caches::Cache;
use p2::P2;
use queues::Queue;

use std::iter::Iterator;
use std::hash::Hash;
use std::cmp::Eq;

use std::mem;

use std::collections::HashMap;

use std::fmt::Debug;

use std::io::stdout;
use std::io::Write;

use std::ptr;
use std::fmt::{Display,Formatter,Result};

use std::rc::Rc;
use std::cell::RefCell;

use queues::request::Request;

#[derive(Debug)]
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
        assert!(self.nb_objects != 0);
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

    fn pop_head(&mut self) -> Option<T> {
        let mut ret = self.head.take();
        if let Some(ref mut old_head) = ret {
            self.nb_objects -= 1;
            self.head = old_head.child.take();
            self.nodes.remove(&old_head.elem);

            match self.head {
                Some(ref mut new_head) => { new_head.parent = ptr::null_mut(); }
                None =>  { self.tail = ptr::null_mut(); }
            }
        }
        ret.map(|x| x.elem)
    }

    fn pop_tail(&mut self) -> Option<T> {
           if !self.tail.is_null() {
               self.nb_objects -= 1;
               let old_tail = self.tail;
               unsafe {
                   assert!((*old_tail).child.is_none());
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
           else {
               assert!(self.head.is_none());
               assert!(self.nb_objects == 0);
               None }
    }

    fn push_head(&mut self, mut new_node: Box<LruNode<T>>) {
        new_node.parent = ptr::null_mut();
        self.nodes.insert(new_node.elem, &mut *new_node);
        if self.tail.is_null() {
            self.tail = &mut *new_node;
        }
        if let Some(ref mut old_head) = self.head {
            old_head.parent = &mut *new_node;
        }
        new_node.child = self.head.take();
        new_node.parent = ptr::null_mut();
        self.head = Some(new_node);
        self.nb_objects += 1;
    }

    fn push_tail(&mut self, mut new_node: Box<LruNode<T>>) {
        self.nodes.insert(new_node.elem, &mut *new_node);
        new_node.child = None;
        let old_tail = self.tail;
        self.tail = &mut *new_node;
        new_node.parent = old_tail;

        if old_tail.is_null() {
            self.head = Some(new_node);
        }
        else {
            unsafe {
                (*old_tail).child = Some(new_node);
            }
        }

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
        while self.nb_objects > self.lru_size {
            self.pop_tail();
        }
    }

    fn resize_and_return (&mut self, new_size : usize) -> Option<LruCache<T>> {
        self.lru_size = new_size;
        if self.nb_objects > new_size {
            let diff = self.nb_objects - new_size;
            Some(self.pop_back_n_nodes(diff))
        }
        else {
            None
        }
    }

    fn pop_back_n_nodes (&mut self, n: usize) -> LruCache<T> {

        assert!(n <= self.nb_objects, "Tried to remove too many objects from Lru Cache");

        let mut new_head : Option<Box<LruNode<T>>> = None;
        let mut new_tail : *mut LruNode<T> = ptr::null_mut();
        let mut new_hash_map: HashMap<T, *mut LruNode<T>> = HashMap::new();

        if n > 0 {
            new_tail = self.tail;

            let mut n = n;
            self.nb_objects -= n;

            unsafe {
                let mut cur_node = self.tail;
                while n > 0 {
                    n -= 1;
                    self.nodes.remove(&(*cur_node).elem);
                    new_hash_map.insert((*cur_node).elem, cur_node);
                    cur_node = (*cur_node).parent;
                }
                if (*cur_node).parent.is_null() { //It's the head
                    self.tail = ptr::null_mut();
                    new_head = self.head.take()
                }
                else {
                    let parent = (*cur_node).parent;
                    self.tail = parent;
                    (*cur_node).parent = ptr::null_mut();
                    new_head = (*parent).child.take()
                }
            }
        }

        LruCache {
            lru_size: n,
            nb_objects: n,
            head: new_head,
            tail: new_tail,
            nodes: new_hash_map
        }
    }

    fn update_all (&mut self, other: LruCache<T>) {
        for elem in other.into_iter() {
            self.update(elem);
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

    fn count(&self) -> usize {
        match self.head {
            None => 0,
            Some(ref node) => {
                let mut count = 1;
                let mut cur_node = node;
                while let Some(ref new_node) = cur_node.child {
                    count += 1;
                    cur_node = new_node
                }
                count
            }
        }
    }

}

impl<T> Display for LruCache<T> where T: Hash+Eq+Copy+Display+Debug {
    fn fmt(&self, f: &mut Formatter) -> Result {
        let mut it = self.iter();
        if let Some(fst) = it.next() {
            try!(write!(f, "{}", fst));
            for item in it {
                try!(write!(f, ",{}", item));
            }
        }
        try!(write!(f,"  Head: {:?}", self.head.as_ref().map(|x| x.elem)));
        Ok(())
    }
}


impl<T> Cache<T> for LruCache<T> where T: Hash+Eq+Copy {

    fn contains (&mut self, entry: &T) -> bool {
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

pub struct P2LruFilter<ContentType, IdType> where
    IdType: Hash+Eq,
    ContentType: Hash+Eq+Copy
{
    latency_limit: f64,
    filter_limit: usize,
    time: f64,
    pit: Pit<IdType>,
    accept: LruCache<ContentType>,
    refuse: LruCache<ContentType>,
    percentile: f64,
    p2_cur: P2,
    p2_train: P2,
    optimize: bool,
}


impl<ContentType, IdType> P2LruFilter<ContentType, IdType> where
    IdType: Hash+Eq+Display+Debug,
    ContentType: Hash+Eq+Copy+Debug+Display
{
    pub fn new (filter_max_size: usize, latency_limit: f64, percentile: f64) -> Self {
        P2LruFilter {
            latency_limit,
            filter_limit: filter_max_size,
            time: 0.,
            pit: Pit::new(),
            accept: LruCache::new(filter_max_size),
            refuse: LruCache::new(0),
            percentile,
            p2_cur: P2::new(percentile),
            p2_train: P2::new(percentile),
            optimize: false,
        }
    }

    fn opt_func(&mut self, cur_value: f64, last_measurement: f64) -> f64 {
        self.p2_cur.new_sample(last_measurement);
        self.p2_train.new_sample(last_measurement);

        if let Some(_) = self.p2_train.get_quantile() {
            self.p2_cur = mem::replace(&mut self.p2_train, P2::new(self.percentile));
        }

        if let Some(curr_quantile) = self.p2_cur.get_quantile() {
            if last_measurement > 0.095 { 
                println!("cur_value: {}, quantile:{}, sample: {}", cur_value, curr_quantile, last_measurement);
            }

            //TODO find something more clever here
            //TODO eg: PI(D) controller?
            let diff = self.latency_limit - curr_quantile;
            if diff > 0. { //Filter must get bigger
                cur_value + (self.filter_limit as f64 - cur_value) * diff / self.latency_limit
            }
            else { //Filter must get smaller
                cur_value * (1. + diff / curr_quantile)
            }
        }
        else {
            cur_value
        }
    }

    fn recompute_filter_pos (&mut self, last_measurement: f64) {
        let old_size = self.accept.lru_size;
        let new_size = self.opt_func(old_size as f64, last_measurement) as usize;
        if new_size != old_size {
            assert!(new_size <= self.filter_limit);
            self.change_filter_pos(new_size);
        }
    }

    pub fn change_filter_pos (&mut self, accept_size: usize) {
        if accept_size > self.accept.lru_size {
            self.accept.resize(accept_size);
            let to_insert = accept_size - self.accept.nb_objects;

            for _ in 0..to_insert {
                match self.refuse.pop_head() {
                    Some(elem) => { self.accept.push_tail(Box::new(LruNode::new(elem))); }
                    None => { break; }
                }
            }

            self.refuse.resize(self.filter_limit - accept_size);
        }
        else if accept_size < self.accept.lru_size {
            assert!(self.filter_limit - accept_size > self.refuse.lru_size);

            self.refuse.resize(self.filter_limit - accept_size);
            while self.accept.nb_objects > accept_size {
                let removed = self.accept.pop_tail();
                let removed = removed.expect(&format!("Tried to remove from cache with {} objects", self.accept.nb_objects));
                self.refuse.update(removed);
            }
            self.accept.resize(accept_size);

            if self.refuse.tail.is_null() && self.refuse.head.is_some() {
                panic!(format!("Refuse size should be {}", self.refuse.nb_objects));
            }
            if self.accept.tail.is_null() && self.accept.head.is_some() {
                panic!(format!("Accept size should be {}", self.accept.nb_objects));
            }

            //let accept_keys: HashSet<ContentType> = HashSet::from_iter(self.accept.nodes.keys().cloned());
            //let refuse_keys: HashSet<ContentType> = HashSet::from_iter(self.refuse.nodes.keys().cloned());
            //assert!(accept_keys.is_disjoint(&refuse_keys));

        }
    }
    
    pub fn set_optimize (&mut self, opt: bool) {
        self.optimize = opt;
    }

    pub fn get_optimize (&self) -> bool {
        self.optimize
    }
}


impl<ContentType, IdType> Cache<(IdType, ContentType)> for P2LruFilter<ContentType, IdType> where
    IdType: Hash+Eq+Copy+Display+Debug,
    ContentType: Hash+Eq+Copy+Display+Debug
{
    fn contains (&mut self, entry: &(IdType, ContentType)) -> bool
    {
        let id = entry.0;
        self.pit.insert(id, self.time);
        self.accept.contains(&entry.1)
    }

    fn update (&mut self, entry: (IdType, ContentType))
    {
        unsafe {
            assert!(self.accept.tail.is_null() || (*self.accept.tail).child.is_none());
            assert!(self.refuse.tail.is_null() || (*self.refuse.tail).child.is_none());
        }
        self.accept.rm_node_if_exists(entry.1);
        let node = Box::new(LruNode::new(entry.1));
        self.accept.push_head(node);
        self.refuse.rm_node_if_exists(entry.1);
        if self.accept.nb_objects > self.accept.lru_size {
            let elem_opt = self.accept.pop_tail();
            if let Some(elem) = elem_opt {
                assert!(self.accept.nodes.get(&elem).is_none());
                self.refuse.update(elem);
            }
        }

        unsafe {
            assert!(self.accept.tail.is_null() || (*self.accept.tail).child.is_none());
            assert!(self.refuse.tail.is_null() || (*self.refuse.tail).child.is_none());
        }

        if let Some(arrival_time) = self.pit.remove(&entry.0) {
            if self.optimize {
                let diff = self.time - arrival_time;
                assert!(!(self.accept.tail.is_null() && self.accept.head.is_some()));
                assert!(!(self.refuse.tail.is_null() && self.refuse.head.is_some()));
                self.recompute_filter_pos(diff);
                assert!(!(self.accept.tail.is_null() && self.accept.head.is_some()));
                assert!(!(self.refuse.tail.is_null() && self.refuse.head.is_some()));
            }
        }
        else {
            panic!("Entry {} not found in PIT", entry.0);
        }
        assert!(!(self.accept.tail.is_null() && self.accept.head.is_some()));
        assert!(!(self.refuse.tail.is_null() && self.refuse.head.is_some()));

    }
}

impl<C,I> Display for P2LruFilter<C,I> where
    I: Hash+Eq+Copy+Display+Debug,
    C: Hash+Eq+Copy+Display+Debug,

{
    fn fmt(&self, f: &mut Formatter) -> Result {
        try!(write!(f, "Accept: {}\n", self.accept));
        try!(write!(f, "Refuse: {}\n", self.refuse));
        //try!(write!(f, "PIT: {:?}\n", self.pit));
        Ok(())
    }
}

pub struct P2LruFilterCont<C,I> where
    I: Hash+Eq+Copy,
    C: Hash+Eq+Copy
{
    f: Rc<RefCell<P2LruFilter<C,I>>>
}

impl<C,I> P2LruFilterCont<C,I> where
    I: Hash+Eq+Copy,
    C: Hash+Eq+Copy
{
    pub fn new(f: Rc<RefCell<P2LruFilter<C,I>>>) -> Self {
        P2LruFilterCont {
            f
        }
    }
}

impl<C,I> Display for P2LruFilterCont<C,I> where
    I: Hash+Eq+Copy+Display+Debug,
    C: Hash+Eq+Copy+Display+Debug,

{
    fn fmt(&self, f: &mut Formatter) -> Result {
        self.f.borrow().fmt(f)
    }
}

impl<C,I> Queue for P2LruFilterCont<C,I> where
    I: Hash+Eq+Copy,
    C: Hash+Eq+Copy
{
    fn arrival(&mut self, _: Request) {}
    fn update_time    (&mut self, time: f64) {
        self.f.borrow_mut().time = time
    }
    fn read_next_exit (&self) -> Option<(f64,&Request)> { None }
    fn pop_next_exit  (&mut self) -> Option<(f64,Request)> { None }
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
