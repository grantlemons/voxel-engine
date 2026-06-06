use crate::contree::gpu_binding::GPUBindable;

use super::{Addr, ChildIndex, Contree, ContreeInner, ContreeLeaf};

bitflags::bitflags! {
    pub struct TreeFlags: u8 {
        const EXISTS = 1 << 0;
        const LEAF = 1 << 1;
        const LIGHT = 1 << 2;
        const _ = 0; // set all other bits to zero
    }
}

impl<T: GPUBindable + Default> Default for Contree<T> {
    fn default() -> Self {
        let mut new = Self {
            center_offset: Default::default(),
            root: Default::default(),
            size: 16,
            inners: Default::default(),
            leaves: Default::default(),
            inner_tombstones: Default::default(),
            leaf_tombstones: Default::default(),
            binding: Default::default(),
        };
        new.root = new.create_root_node();
        new
    }
}

impl<T: GPUBindable> Contree<T> {
    pub fn new(binding: T) -> Self {
        let mut new = Self {
            center_offset: Default::default(),
            root: Default::default(),
            size: 16,
            inners: Default::default(),
            leaves: Default::default(),
            inner_tombstones: Default::default(),
            leaf_tombstones: Default::default(),
            binding,
        };
        new.root = new.create_root_node();
        new
    }
}

impl<T: GPUBindable> Contree<T> {
    pub(super) fn create_root_node(&mut self) -> Addr {
        let new_node = ContreeInner {
            contains: 0,
            leaf: 0,
            light: 0,
            children: [0; 64],
        };
        let addr = match self.inner_tombstones.pop() {
            Some(addr) => {
                self.inners[addr as usize] = new_node;
                addr
            }
            None => {
                self.inners.push(new_node);
                (self.inners.len() - 1) as Addr
            }
        };
        self.binding.write_inner(addr, &[new_node]);
        addr
    }

    pub(super) fn create_inner_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
        let addr = self.create_root_node();
        self.inners[parent as usize].children[index] = addr;
        self.update_parent_bitflags(parent, index, TreeFlags::EXISTS);
        addr
    }

    pub(super) fn create_leaf_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
        let new_node = ContreeLeaf {
            contains: 0,
            light: 0,
            children: [0; 64],
        };
        let addr = match self.leaf_tombstones.pop() {
            Some(addr) => {
                self.leaves[addr as usize] = new_node;
                addr
            }
            None => {
                self.leaves.push(new_node);
                (self.leaves.len() - 1) as Addr
            }
        };
        self.inners[parent as usize].children[index] = addr;
        self.update_parent_bitflags(parent, index, TreeFlags::EXISTS | TreeFlags::LEAF);

        self.binding.write_leaf(addr, &[new_node]);
        addr
    }

    fn update_parent_bitflags(&mut self, parent: Addr, child: ChildIndex, flags: TreeFlags) {
        let parent_node = &mut self.inners[parent as usize];
        parent_node.contains |= (flags.contains(TreeFlags::EXISTS) as u64) << child;
        parent_node.leaf |= (flags.contains(TreeFlags::LEAF) as u64) << child;
        parent_node.light |= (flags.contains(TreeFlags::LIGHT) as u64) << child;

        self.binding.write_inner(parent, &[*parent_node]);
    }
}
