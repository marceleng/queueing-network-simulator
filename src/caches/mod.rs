pub mod lru_cache;

pub trait Cache<T> {
    fn contains (&self, entry: &T) -> bool;
    fn update (&mut self, entry: T);
}
