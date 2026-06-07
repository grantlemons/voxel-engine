use super::{Addr, ContreeInner};
use crate::ContreeLeaf;

pub trait GPUBindable: std::fmt::Debug + Clone {
    fn write_inner(&self, addr: Addr, data: &[ContreeInner]);
    fn write_leaf(&self, addr: Addr, data: &[ContreeLeaf]);
}

#[derive(Debug, Clone, Default)]
pub struct DummyBinding;
impl GPUBindable for DummyBinding {
    fn write_inner(&self, _: Addr, _: &[ContreeInner]) {}
    fn write_leaf(&self, _: Addr, _: &[ContreeLeaf]) {}
}
