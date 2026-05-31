use super::{
    Addr, ChildIndex, ContreeInner, ContreeLeaf, FindResult, gpu_binding::GPUBinding, util::*,
};
use glam::{UVec3, Vec3};
use std::fmt::Display;

mod tests;

bitflags::bitflags! {
    struct TreeFlags: u8 {
        const EXISTS = 1 << 0;
        const LEAF = 1 << 1;
        const LIGHT = 1 << 2;
        const _ = 0; // set all other bits to zero
    }
}

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
        new.root = new.create_root_node();
        new
    }
}

impl Contree {
    fn normalize(&self, p: Vec3) -> UVec3 {
        (p - self.center_offset + (self.size as f32 / 2.))
            .round()
            .as_uvec3()
    }

    fn in_bounds(&self, p: Vec3) -> bool {
        fn svo_abs(v: f32) -> f32 {
            if v < 0. { -v - 1. } else { v }
        }
        (p - self.center_offset)
            .map(svo_abs)
            .round()
            .as_uvec3()
            .max_element()
            <= self.size / 2
    }

    fn create_root_node(&mut self) -> Addr {
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

    fn create_inner_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
        let addr = self.create_root_node();
        self.inners[parent as usize].children[index] = addr;
        self.update_parent_bitflags(parent, index, TreeFlags::EXISTS);
        addr
    }

    fn create_leaf_node(&mut self, parent: Addr, index: ChildIndex) -> Addr {
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

        self.gpu.write_leaf(addr, &[new_node]);
        addr
    }

    fn update_parent_bitflags(&mut self, parent: Addr, child: ChildIndex, flags: TreeFlags) {
        let parent_node = &mut self.inners[parent as usize];
        parent_node.contains |= (flags.contains(TreeFlags::EXISTS) as u64) << child;
        parent_node.leaf |= (flags.contains(TreeFlags::LEAF) as u64) << child;
        parent_node.light |= (flags.contains(TreeFlags::LIGHT) as u64) << child;

        self.gpu.write_inner(parent, &[*parent_node]);
    }

    /// Grow upward until the position is in bounds
    fn grow_to_accomodate(&mut self, pos: Vec3) {
        while !self.in_bounds(pos) {
            let new_root = self.create_root_node();
            self.size *= 4;

            let new_center = Vec3::ZERO;
            let old_root_coords = self.center_offset;
            self.center_offset = new_center;
            let old_root_new_index = to_base_64(morton_code(self.normalize(old_root_coords)))
                .last()
                .unwrap();

            // set current node as child of new node
            self.inners[new_root as usize].children[old_root_new_index] = self.root;
            self.gpu
                .write_inner(new_root, &[self.inners[new_root as usize]]);

            self.root = new_root;
            todo!("Contree cannot grow yet!")
        }
    }

    pub fn insert(&mut self, pos: Vec3, material: u8) -> FindResult {
        self.grow_to_accomodate(pos);

        let FindResult {
            leaf_address,
            traversal_stack,
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
        FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size: 1,
        }
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
                1 => leaf_addr = self.create_leaf_node(parent, *child_index),
                _ => parent_addrs.push(self.create_inner_node(parent, *child_index)),
            }
        }
        unreachable!("Never reached bottom of traversal stack!")
    }

    pub fn find(&self, pos: Vec3, given_parent_addrs: &[Addr]) -> FindResult {
        let mut traversal_stack: Vec<ChildIndex> =
            to_base_64(morton_code(self.normalize(pos))).collect();

        for _ in given_parent_addrs {
            traversal_stack.pop();
        }

        let mut parent_addrs = given_parent_addrs.to_vec();
        parent_addrs.push(self.root);
        let mut current = self.inners[self.root as usize];
        for _ in 0..(traversal_stack.len()) {
            let index = traversal_stack.last().unwrap();
            let child_addr = current.children[*index] as Addr;

            let child_exists = current.contains & (1 << index) != 0;
            let child_leaf = current.leaf & (1 << index) != 0;

            if child_exists && child_leaf {
                // leaf node contains this coordinate
                // this does not mean that something exists at this coordinate
                traversal_stack.pop();

                // check if something exists at pos
                let leaf = self.leaves[child_addr as usize];
                let index = traversal_stack.last().unwrap();
                let contains = leaf.contains & (1 << index) != 0;

                let node_size = if contains {
                    self.size >> ((parent_addrs.len() as u32 + 1) * 2)
                } else {
                    self.size >> ((parent_addrs.len() as u32) * 2)
                };

                return FindResult {
                    leaf_address: Some(child_addr),
                    traversal_stack,
                    node_size,
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
                    node_size: self.size >> ((parent_addrs.len() as u32 - 1) * 2),
                    parent_addrs,
                };
            }
        }

        unreachable!()
    }

    // When moving in a node, unless you know it has no children, you can only move 1/4 at a time
    fn max_travel_distance(
        &self,
        leaf_address: Option<Addr>,
        parent_addrs: &[Addr],
        node_size: u32,
    ) -> u32 {
        if leaf_address.is_none()
            && let Some(&parent_addr) = parent_addrs.last()
            && self.inners[parent_addr as usize].contains == 0
        {
            node_size
        } else {
            node_size >> 2
        }
    }

    pub fn raycast(&self, pos: Vec3, dir: Vec3) -> Option<Vec3> {
        let mut p = pos;

        if !self.in_bounds(p) {
            let norm_dir = dir.normalize();

            let move_distance = [
                self.center_offset + (self.size as f32) / 2. - 0.5,
                -self.center_offset + (self.size as f32) / 2. - 0.5,
            ]
            .iter()
            .filter_map(|bound: &Vec3| {
                let max_t = (bound - p) / norm_dir;

                // Maximum element of max_t, ignoring inf, -inf, and NaN values
                // This should find the outside bound of the tree
                max_t
                    .to_array()
                    .into_iter()
                    .filter(|&x| f32::is_normal(x))
                    .filter(|&x| f32::is_sign_positive(x))
                    .reduce(f32::max)
            })
            // Pick the closer bound to move to
            .reduce(f32::min);

            p += move_distance? * norm_dir;
        }

        let mut find_p = p;
        let mut i = 0;
        while self.in_bounds(p) && i < 50 {
            let FindResult {
                leaf_address,
                traversal_stack,
                node_size,
                parent_addrs,
                ..
            } = self.find(find_p, &[]);

            // break if hit solid
            if let Some(laddr) = leaf_address
                && let Some(&cidx) = traversal_stack.last()
                && self.leaves[laddr as usize].contains & (0b1 << cidx) != 0
                && self.leaves[laddr as usize].children[cidx] != 0
            {
                return Some(p);
            }

            let child_size =
                self.max_travel_distance(leaf_address, &parent_addrs, node_size) as f32;
            let dir_sign = dir.map(|v| if v == 0. { 0. } else { v.signum() });

            let bspace_p = p + 0.5 - self.center_offset;
            let bspace_boundary =
                child_size * round_in_dir(bspace_p / child_size + dir_sign / 2., dir);
            let pspace_boundary = bspace_boundary - 0.5 + self.center_offset;

            // Maximum t before hitting boundary on each axis
            let norm_dir = dir.normalize();
            let max_t = (pspace_boundary - p) / norm_dir;

            // Minimum element of max_t, ignoring inf, -inf, and NaN values
            let move_distance = max_t.abs().to_array().into_iter().reduce(f32::min).unwrap();

            p += move_distance * norm_dir; // jump to boundary

            find_p = p + (dir_sign * 0.001);
            i += 1;
        }
        None
    }
}

impl Display for Contree {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(
            f,
            "digraph {{
\tnewrank=true;
\trankdir=LR;"
        )?;

        let mut stack = vec![self.root];

        while let Some(addr) = stack.pop() {
            let cur = self.inners[addr as usize];
            for i in 0..64 {
                if (cur.contains & (0b1 << i)) != 0 {
                    if (cur.leaf & (0b1 << i)) != 0 {
                        writeln!(
                            f,
                            "\t{} -> \"leaf {}\" [label=<{}>]",
                            addr, cur.children[i], i
                        )?;

                        let leaf_addr = cur.children[i];
                        for j in 0..64 {
                            if (self.leaves[leaf_addr as usize].contains & (0b1 << j)) != 0 {
                                writeln!(
                                    f,
                                    "\t\"leaf {}\" -> \"mat {}\" [label=<{}>]",
                                    leaf_addr, self.leaves[leaf_addr as usize].children[j], j
                                )?;
                            }
                        }
                    } else {
                        writeln!(f, "\t{} -> {} [label=<{}>]", addr, cur.children[i], i)?;
                        stack.push(cur.children[i]);
                    }
                }
            }
        }

        writeln!(f, "}}")
    }
}
