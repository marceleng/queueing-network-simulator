pub mod lru_cache;

pub trait Cache<T> {
    fn contains (&mut self, entry: &T) -> bool;
    fn update (&mut self, entry: T);
}
