#![allow(unused)]
use bytemuck::{Pod, Zeroable};
use glam::{IVec3, UVec3, Vec3};

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

type ChildIndex = usize;
type Addr = usize;

#[derive(Debug, Clone, Default)]
struct Contree {
    center_offset: Vec3,
    root: Addr,
    /// Distance from center to face
    size: u32,
    inners: Vec<ContreeInner>,
    leaves: Vec<ContreeLeaf>,
    inner_tombstones: Vec<Addr>,
    leaf_tombstones: Vec<Addr>,
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
        return x;
    }
    (interleave(norm_p.x) << 2) | (interleave(norm_p.y) << 1) | interleave(norm_p.z)
}

struct FindResult {
    leaf_address: Option<Addr>,
    traversal_stack: Vec<ChildIndex>,
    parent_addrs: Vec<Addr>,
}

impl Contree {
    fn normalize(&self, p: Vec3) -> UVec3 {
        (p - self.center_offset + Vec3::splat(self.size as f32))
            .round()
            .as_uvec3()
    }

    fn svo_abs(v: f32) -> f32 {
        if v < 0. { -v - 1. } else { v }
    }

    fn in_bounds(&self, p: Vec3) -> bool {
        ((p - self.center_offset)
            .map(Self::svo_abs)
            .round()
            .max_element() as u32)
            < self.size
    }

    fn new_inner_node(&mut self, parent: Addr, child: ChildIndex) -> Addr {
        let new_node = ContreeInner {
            contains: 0,
            leaf: 0,
            light: 0,
            children: [0; 64],
        };
        let addr = match self.inner_tombstones.pop() {
            Some(addr) => {
                self.inners[addr] = new_node;
                addr
            }
            None => {
                self.inners.push(new_node);
                self.inners.len() - 1
            }
        };
        self.inners[parent].children[child] = addr as u32;
        self.update_parent_bitflags(parent, child, true, false, false);
        addr
    }

    fn new_leaf_node(&mut self, parent: Addr, child: ChildIndex) -> Addr {
        let new_node = ContreeLeaf {
            contains: 0,
            light: 0,
            children: [0; 64],
        };
        let addr = match self.leaf_tombstones.pop() {
            Some(addr) => {
                self.leaves[addr] = new_node;
                addr
            }
            None => {
                self.leaves.push(new_node);
                self.leaves.len() - 1
            }
        };
        self.inners[parent].children[child] = addr as u32;
        self.update_parent_bitflags(parent, child, true, true, false);
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
        let mask = (1 as u64) << child;
        self.inners[parent].contains &= !mask;
        self.inners[parent].contains |= (exists as u64) << child;
        self.inners[parent].leaf &= !mask;
        self.inners[parent].leaf |= (leaf as u64) << child;
        self.inners[parent].light &= !mask;
        self.inners[parent].light |= (light as u64) << child;
    }

    fn insert(&mut self, pos: Vec3, material: u8) -> Vec<Addr> {
        if self.in_bounds(pos) {
            let FindResult {
                leaf_address,
                mut traversal_stack,
                mut parent_addrs,
            } = self.find(pos, &[]);
            match leaf_address {
                Some(addr) => {
                    let leaf = self
                        .leaves
                        .get_mut(addr)
                        .expect("Leaf node does not exist!");

                    let child_index = *traversal_stack.last().unwrap();
                    leaf.children[child_index] = material;
                    leaf.contains |= 1 << child_index;
                }
                None => {
                    let (leaf_addr, child_index) =
                        self.add_parents(&mut traversal_stack, &mut parent_addrs);

                    let leaf = self
                        .leaves
                        .get_mut(leaf_addr)
                        .expect("Leaf node does not exist!");

                    leaf.children[child_index] = material;
                    leaf.contains |= 1 << child_index;
                }
            }
            return parent_addrs;
        } else {
            // TODO: Expand contree
            let contains = self.inners[self.root].contains;
            todo!()
        }
    }

    fn add_parents(
        &mut self,
        traversal_stack: &mut Vec<ChildIndex>,
        parent_addrs: &mut Vec<Addr>,
    ) -> (Addr, ChildIndex) {
        let mut leaf_addr = 0;
        for (i, child_index) in traversal_stack.clone().iter().enumerate().rev() {
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
        if self.inners.is_empty() {
            return FindResult {
                leaf_address: None,
                traversal_stack,
                parent_addrs,
            };
        }

        parent_addrs.push(self.root);
        let mut current = self.inners[self.root];
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
                    parent_addrs,
                };
            }
            if child_exists {
                traversal_stack.pop();
                parent_addrs.push(child_addr);
                current = self.inners[child_addr];
            } else {
                return FindResult {
                    leaf_address: None,
                    traversal_stack,
                    parent_addrs,
                };
            }
        }

        FindResult {
            leaf_address: None,
            traversal_stack,
            parent_addrs,
        }
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
        } = contree.find(p, &[]);

        assert!(leaf_address.is_none());
        assert_eq!(traversal_stack, &[5, 28]);
        assert!(parent_addrs.is_empty());
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
            size: 8,
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
        } = contree.find(p, &[]);

        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_stack.as_slice(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0]);
    }

    #[test]
    fn insert_traverse_tiny() {
        let p = Vec3::new(0., 0., 0.);
        let mut contree = Contree {
            root: 0,
            size: 16,
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

        let FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
        } = contree.find(p, &[]);

        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_stack.as_slice(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0, 1]);
    }
}
