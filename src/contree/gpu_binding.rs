use bytemuck::cast_slice;
use flume::Sender;

use super::{Addr, ContreeInner};
use crate::{contree::ContreeLeaf, renderer::BufferWriteCommand};

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

#[derive(Debug, Clone)]
pub struct ChannelBinding {
    pub writer: Sender<BufferWriteCommand>,
    pub inner_buffer: wgpu::Buffer,
    pub leaf_buffer: wgpu::Buffer,
}

impl GPUBindable for ChannelBinding {
    fn write_inner(&self, addr: Addr, data: &[ContreeInner]) {
        let _ = self.writer.send(BufferWriteCommand {
            target_buffer: self.inner_buffer.clone(),
            offset: addr as u64 * size_of::<ContreeInner>() as u64,
            new_data: cast_slice(data).to_vec(),
        });
    }

    fn write_leaf(&self, addr: Addr, data: &[ContreeLeaf]) {
        let _ = self.writer.send(BufferWriteCommand {
            target_buffer: self.inner_buffer.clone(),
            offset: addr as u64 * size_of::<ContreeLeaf>() as u64,
            new_data: cast_slice(data).to_vec(),
        });
    }
}
