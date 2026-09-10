use crate::*;
use std::vec::Vec;

#[extern_spec]
impl<T, A: core::alloc::Allocator> Vec<T, A> {
    #[trusted]
    #[pure]
    fn len(&self) -> usize;
}
