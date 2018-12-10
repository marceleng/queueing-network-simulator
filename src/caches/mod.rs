pub mod lru_cache;
pub mod abf_cache;
pub mod abf_fpga_cache;

pub trait Cache<T> {
    fn contains (&mut self, entry: &T) -> bool;
    fn update (&mut self, entry: T);
}
