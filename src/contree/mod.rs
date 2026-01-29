#![allow(unused)]
use std::fmt::Display;

use bytemuck::{Pod, Zeroable};
use glam::{IVec3, UVec3, Vec3};

use crate::contree::gpu_binding::GPUBinding;

mod gpu_binding;

// 80 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeLeaf {
    contains: u64,
    light: u64,
    children: [u8; 64],
}

// 280 bytes
#[repr(C, align(4))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeInner {
    contains: u64,
    leaf: u64,
    light: u64,
    children: [u32; 64],
}

#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Material {
    color: [f32; 4],
    reflectivity: f32,
    padding: [u8; 12],
}

type ChildIndex = usize;

/// Address in terms of data type, not bytes
type Addr = u32;

#[derive(Debug, Clone)]
pub struct Contree {
    pub center_offset: Vec3,
    pub root: Addr,
    /// Distance from face to face
    pub size: u32,
    inners: Vec<ContreeInner>,
    leaves: Vec<ContreeLeaf>,
    inner_tombstones: Vec<Addr>,
    leaf_tombstones: Vec<Addr>,
    gpu: GPUBinding,
}

impl Default for Contree {
    fn default() -> Self {
        let mut new = Self {
            center_offset: Default::default(),
            root: Default::default(),
            size: 16,
            inners: Default::default(),
            leaves: Default::default(),
            inner_tombstones: Default::default(),
            leaf_tombstones: Default::default(),
            gpu: Default::default(),
        };
        new.new_root_node();
        new
    }
}

fn morton_code(norm_p: UVec3) -> u64 {
    fn interleave(val: u32) -> u64 {
        // Magic number bit-fuckery
        let magic_numbers = [
            0x1249249249249249,
            0x10c30c30c30c30c3,
            0x100f00f00f00f00f,
            0x1f0000ff0000ff,
            0x1f00000000ffff,
        ];
        // Only the first 21 bits are used
        let mut x = (val & 0x1fffff) as u64;
        for (i, mn) in magic_numbers.iter().enumerate().rev() {
            x = (x | x << (1 << (i + 1))) & mn;
        }
        x
    }
    (interleave(norm_p.x) << 2) | (interleave(norm_p.y) << 1) | interleave(norm_p.z)
}

#[derive(Debug)]
struct FindResult {
    leaf_address: Option<Addr>,
    traversal_stack: Vec<ChildIndex>,
    parent_addrs: Vec<Addr>,
    /// Distance from face to face
    node_size: u32,
}

impl Contree {
    fn normalize(&self, p: Vec3) -> UVec3 {
        (p - self.center_offset + (self.size as f32 / 2.) + 0.5)
            .trunc()
            .as_uvec3()
    }

    fn svo_abs(v: f32) -> f32 {
        if v < 0. { -v - 1. } else { v }
    }

    fn in_bounds(&self, p: Vec3) -> bool {
        (p - self.center_offset)
            .map(Self::svo_abs)
            .round()
            .as_uvec3()
            .max_element()
            < self.size / 2
    }

    fn new_root_node(&mut self) -> Addr {
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
        self.gpu.write_inner(addr, &[new_node]);
        addr
    }

    fn new_inner_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
        let addr = self.new_root_node();
        self.inners[parent as usize].children[index] = addr;
        self.update_parent_bitflags(parent, index, true, false, false);
        addr
    }

    fn new_leaf_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
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
        self.update_parent_bitflags(parent, index, true, true, false);

        self.gpu.write_leaf(addr, &[new_node]);
        addr
    }

    fn update_parent_bitflags(
        &mut self,
        parent: Addr,
        child: ChildIndex,
        exists: bool,
        leaf: bool,
        light: bool,
    ) {
        let mask = (1_u64) << child;

        let parent_node = &mut self.inners[parent as usize];
        parent_node.contains &= !mask;
        parent_node.contains |= (exists as u64) << child;
        parent_node.leaf &= !mask;
        parent_node.leaf |= (leaf as u64) << child;
        parent_node.light &= !mask;
        parent_node.light |= (light as u64) << child;

        self.gpu.write_inner(parent, &[*parent_node]);
    }

    pub fn insert(&mut self, pos: Vec3, material: u8) -> Vec<Addr> {
        // Grow upward until the position is in bounds
        while !self.in_bounds(pos) {
            let new_root = self.new_root_node();
            let self_index = 0;
            self.inners[new_root as usize].children[self_index] = self.root;
            self.root = new_root;
            self.size *= 4;

            self.gpu
                .write_inner(new_root, &[self.inners[new_root as usize]]);
            todo!()
        }

        let FindResult {
            leaf_address,
            mut traversal_stack,
            mut parent_addrs,
            ..
        } = self.find(pos, &[]);
        match leaf_address {
            Some(leaf_addr) => {
                let leaf = self
                    .leaves
                    .get_mut(leaf_addr as usize)
                    .expect("Leaf node does not exist!");

                let child_index = *traversal_stack.last().unwrap();
                leaf.children[child_index] = material;
                leaf.contains |= 1 << child_index;
                self.gpu.write_leaf(leaf_addr, &[*leaf]);
            }
            None => {
                let (leaf_addr, child_index) =
                    self.add_parents(&traversal_stack, &mut parent_addrs);

                let leaf = self
                    .leaves
                    .get_mut(leaf_addr as usize)
                    .expect("Leaf node does not exist!");

                leaf.children[child_index] = material;
                leaf.contains |= 1 << child_index;
                self.gpu.write_leaf(leaf_addr, &[*leaf]);
            }
        }
        parent_addrs
    }

    fn add_parents(
        &mut self,
        traversal_stack: &[ChildIndex],
        parent_addrs: &mut Vec<Addr>,
    ) -> (Addr, ChildIndex) {
        let mut leaf_addr = 0;
        for (i, child_index) in traversal_stack.iter().enumerate().rev() {
            let parent: Addr = *parent_addrs.last().expect("No root!");
            match i {
                0 => return (leaf_addr, *child_index),
                1 => leaf_addr = self.new_leaf_node(parent, *child_index),
                _ => parent_addrs.push(self.new_inner_node(parent, *child_index)),
            }
        }
        unreachable!("Never reached bottom of traversal stack!")
    }

    fn to_base_64(code: u64) -> Vec<ChildIndex> {
        let mut digits = Vec::new();
        let mut n = code;
        if n == 0 {
            digits.push(0);
            return digits;
        }
        while n != 0 {
            digits.push((n as ChildIndex) & 0b111111);
            n >>= 6;
        }
        digits
    }

    fn find(&self, pos: Vec3, given_parent_addrs: &[Addr]) -> FindResult {
        let code = morton_code(self.normalize(pos));
        let mut traversal_stack = Self::to_base_64(code);

        for i in given_parent_addrs {
            traversal_stack.pop();
        }

        let mut parent_addrs = given_parent_addrs.to_vec();
        parent_addrs.push(self.root);
        let mut current = self.inners[self.root as usize];
        for i in 0..(traversal_stack.len()) {
            let index = traversal_stack.last().unwrap();
            let child_addr = current.children[*index] as Addr;

            let child_exists = current.contains & (1 << index) != 0;
            let child_leaf = current.leaf & (1 << index) != 0;

            if child_exists && child_leaf {
                traversal_stack.pop();
                return FindResult {
                    leaf_address: Some(child_addr),
                    traversal_stack,
                    node_size: self.size / 4_u32.pow(parent_addrs.len() as u32 + 1),
                    parent_addrs,
                };
            } else if child_exists {
                traversal_stack.pop();
                parent_addrs.push(child_addr);
                current = self.inners[child_addr as usize];
            } else {
                return FindResult {
                    leaf_address: None,
                    traversal_stack,
                    node_size: self.size / 4_u32.pow(parent_addrs.len() as u32 - 1),
                    parent_addrs,
                };
            }
        }

        unreachable!()
    }

    pub fn raycast(&self, pos: Vec3, dir: Vec3) -> Vec3 {
        let mut p = pos;
        let mut i = 0;
        while self.in_bounds(p) && i < 50 {
            dbg!(p);
            let FindResult {
                node_size,
                parent_addrs,
                ..
            } = self.find(p, &[]);

            let norm_dir = dir; // .normalize_or_zero();
            let dir_signs = norm_dir.signum();

            let boundary = node_size as f32
                * ((p + 0.5 + (node_size as f32 * norm_dir) / 2.) / node_size as f32).round()
                - 0.5;

            let max_t = (boundary - p) / norm_dir;
            p += max_t.min_element() * norm_dir;
            i += 1;
        }

        todo!()
    }
}

#[cfg(test)]
mod tests {
    use glam::{UVec3, Vec3};

    use crate::contree::{Contree, ContreeInner, ContreeLeaf, FindResult, morton_code};

    #[test]
    fn morton_code_example() {
        let code = morton_code(UVec3::new(5, 8, 9));
        assert_eq!(code, 0b011100000101);
    }

    #[test]
    fn morton_code_zero() {
        let code = morton_code(UVec3::new(0, 0, 0));
        let index = Contree::to_base_64(code);
        assert_eq!(code, 0);
        assert_eq!(index, &[0]);
    }

    #[test]
    fn traverse_empty() {
        let p = Vec3::new(5., 8., 9.);
        let contree = Contree::default();
        let FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        assert!(leaf_address.is_none());
        assert_eq!(traversal_stack, &[5, 36, 3]);
        assert_eq!(parent_addrs.as_slice(), &[0]);
        assert_eq!(node_size, 16);
    }

    #[test]
    fn traverse_tiny() {
        let p = Vec3::new(0., 0., 0.);

        let mut inner_children = [0; 64];
        let mut leaf_children = [0; 64];
        inner_children[56] = 0;
        leaf_children[0] = 10;
        let contree = Contree {
            root: 0,
            size: 16,
            inners: vec![ContreeInner {
                contains: 1 << 56,
                leaf: 1 << 56,
                light: 0,
                children: inner_children,
            }],
            leaves: vec![ContreeLeaf {
                contains: 1 << 0,
                light: 0,
                children: leaf_children,
            }],
            ..Default::default()
        };

        let FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_stack.as_slice(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0]);
        assert_eq!(node_size, 1);
    }

    #[test]
    fn insert_traverse_tiny() {
        let p = Vec3::new(0., 0., 0.);
        let mut contree = Contree {
            root: 0,
            size: 64,
            inners: vec![ContreeInner {
                contains: 0,
                leaf: 0,
                light: 0,
                children: [0; 64],
            }],
            leaves: Vec::new(),
            ..Default::default()
        };
        contree.insert(p, 10);
        contree.insert(Vec3::new(4., 4., 4.), 5);

        let FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        contree.raycast(Vec3::new(0., 0., 0.), -Vec3::splat(1.));

        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_stack.as_slice(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0, 1]);
        assert_eq!(node_size, 1);
    }

    #[test]
    fn insert_many_no_grow() {
        let p = Vec3::new(0., 0., 0.);
        let mut contree = Contree {
            root: 0,
            size: 4_u32.pow(3),
            inners: vec![ContreeInner {
                contains: 0,
                leaf: 0,
                light: 0,
                children: [0; 64],
            }],
            leaves: Vec::new(),
            ..Default::default()
        };
        contree.insert(p, 10);
        contree.insert(Vec3::new(0., 0., 1.), 1);
        contree.insert(Vec3::new(0., 1., 0.), 2);
        contree.insert(Vec3::new(1., 0., 0.), 3);
        contree.insert(Vec3::new(-10., 10., 10.), 4);
        contree.insert(Vec3::new(-10., 0., 0.), 5);
        contree.insert(Vec3::new(-10., -10., 0.), 6);
    }
}

impl Display for Contree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "digraph {{
\tnewrank=true;
\trankdir=LR;"
        );

        let mut stack = vec![self.root];

        while !stack.is_empty() {
            let addr = stack.pop().unwrap();
            let cur = self.inners[addr as usize];
            for i in 0..64 {
                if (cur.contains & (0b1 << i)) != 0 {
                    if (cur.leaf & (0b1 << i)) != 0 {
                        writeln!(
                            f,
                            "\t{} -> \"leaf {}\" [label=<{}>]",
                            addr, cur.children[i], i
                        );

                        let leaf_addr = cur.children[i];
                        for j in 0..64 {
                            if (self.leaves[leaf_addr as usize].contains & (0b1 << j)) != 0 {
                                writeln!(
                                    f,
                                    "\t\"leaf {}\" -> \"mat {}\" [label=<{}>]",
                                    leaf_addr, self.leaves[leaf_addr as usize].children[j], j
                                );
                            }
                        }
                    } else {
                        writeln!(f, "\t{} -> {} [label=<{}>]", addr, cur.children[i], i);
                        stack.push(cur.children[i]);
                    }
                }
            }
        }

        writeln!(f, "}}")
    }
}
