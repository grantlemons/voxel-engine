use bytemuck::{Pod, Zeroable};
use glam::Vec3;
use std::cmp::Ordering;

// 80 bytes
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeLeaf {
    contains: u64,
    light: u64,
    children: [u8; 64],
}

// 288 bytes
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeInner {
    contains: u64,
    leaf: u64,
    light: u64,
    children: [u32; 64],
    default_material: u32,
    padding: [u8; 4],
}

#[derive(Debug, Clone)]
struct Contree {
    root: u32,
    center: Vec3,
    /// Distance from center to any face
    size: u32,
    inners: Vec<ContreeInner>,
    leaves: Vec<ContreeLeaf>,
    inner_tombstones: Vec<usize>,
    leaf_tombstones: Vec<usize>,
}

impl Contree {
    fn add_leaf_node(&mut self, parent: usize, child_num: usize, node: ContreeLeaf) {
        let child = match self.leaf_tombstones.pop() {
            Some(child) => {
                self.leaves[child] = node;
                child
            }
            None => {
                let child = self.leaves.len();
                self.leaves.push(node);
                child
            }
        };

        self.inners[parent].children[child_num] = child as u32;
        self.update_parent_bitflags(parent, child_num, true, true, node.light != 0);
    }

    fn add_inner_node(&mut self, parent: usize, child_num: usize, node: ContreeInner) {
        let child = match self.inner_tombstones.pop() {
            Some(child) => {
                self.inners[child] = node;
                child
            }
            None => {
                let child = self.inners.len();
                self.inners.push(node);
                child
            }
        };

        self.inners[parent].children[child_num] = child as u32;
        self.update_parent_bitflags(parent, child_num, true, false, node.light != 0);
    }

    fn find(&self, pos: Vec3) -> (Option<usize>, usize, Vec<usize>) {
        fn binary(
            center: f32,
            size: u32,
            pos: f32,
            range: (usize, usize),
        ) -> ((usize, usize), f32) {
            let ordering = pos.total_cmp(&center);
            let new_center = match ordering {
                Ordering::Less => center - (size / 2) as f32,
                Ordering::Equal => panic!("Equal coordinates in contree!"),
                Ordering::Greater => center + (size / 2) as f32,
            };
            let new_range = match ordering {
                Ordering::Less => (range.0, range.1 - (range.1 - range.0) / 2),
                Ordering::Equal => panic!("Equal coordinates in contree!"),
                Ordering::Greater => (range.0 + (range.1 - range.0) / 2, range.1),
            };

            (new_range, new_center)
        }

        fn child_index(center: Vec3, size: u32, pos: Vec3) -> (usize, Vec3) {
            let mut size = size;
            let mut current_pos = pos;
            let mut range = (0, 63);
            for _ in 0..2 {
                (range, current_pos.x) = binary(center.x, size, pos.x, range);
                (range, current_pos.y) = binary(center.y, size, pos.y, range);
                (range, current_pos.z) = binary(center.z, size, pos.z, range);
                size /= 2;
            }

            return (range.0, current_pos);
        }

        let mut res = Vec::new();
        let mut current_size = self.size;
        let mut current_center = self.center;
        let mut current_addr = 0;
        loop {
            res.push(current_addr);
            let (idx, child_center) = child_index(current_center, current_size, pos);

            let current = self.inners[current_addr];
            let child_addr = current.children[idx] as usize;

            let child_exists = current.contains & (1 as u64) << idx != 0;
            let child_is_leaf = current.leaf & (1 as u64) << idx != 0;
            if !child_exists {
                return (None, idx, res);
            }
            if child_is_leaf {
                return (Some(child_addr), idx, res);
            }

            current_size /= 4;
            current_center = child_center;
            current_addr = child_addr;
        }
    }

    fn update_parent_bitflags(
        &mut self,
        parent: usize,
        child_num: usize,
        exists: bool,
        leaf: bool,
        light: bool,
    ) {
        let mask = (1 as u64) << child_num;
        self.inners[parent].contains &= !mask;
        self.inners[parent].contains |= (exists as u64) << child_num;
        self.inners[parent].leaf &= !mask;
        self.inners[parent].leaf |= (leaf as u64) << child_num;
        self.inners[parent].light &= !mask;
        self.inners[parent].light |= (light as u64) << child_num;
    }
}
