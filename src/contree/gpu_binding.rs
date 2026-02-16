use bytemuck::cast_slice;
use flume::Sender;

use super::{Addr, ContreeInner};
use crate::{contree::ContreeLeaf, renderer::BufferWriteCommand};

#[derive(Debug, Clone, Default)]
#[non_exhaustive]
pub enum GPUBinding {
    #[default]
    Dummy,
    Channel {
        writer: Sender<BufferWriteCommand>,
        inner_buffer: wgpu::Buffer,
        leaf_buffer: wgpu::Buffer,
    },
}

impl GPUBinding {
    pub fn write_inner(&self, addr: Addr, data: &[ContreeInner]) {
        match self {
            GPUBinding::Dummy => {}
            GPUBinding::Channel {
                writer,
                inner_buffer,
                ..
            } => {
                let _ = writer.send(BufferWriteCommand {
                    target_buffer: inner_buffer.clone(),
                    offset: addr as u64 * size_of::<ContreeInner>() as u64,
                    new_data: cast_slice(data).to_vec(),
                });
            }
            _ => {}
        }
    }

    pub fn write_leaf(&self, addr: Addr, data: &[ContreeLeaf]) {
        match self {
            GPUBinding::Dummy => {}
            GPUBinding::Channel {
                writer,
                inner_buffer,
                ..
            } => {
                let _ = writer.send(BufferWriteCommand {
                    target_buffer: inner_buffer.clone(),
                    offset: addr as u64 * size_of::<ContreeLeaf>() as u64,
                    new_data: cast_slice(data).to_vec(),
                });
            }
            _ => {}
        }
    }
}
