use bytemuck::{Pod, Zeroable};

// 80 bytes
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeLeaf {
    contains: u64,
    light: u64,
    children: [u32; 16],
}

// 288 bytes
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, Pod, Zeroable)]
struct ContreeInner {
    contains: u64,
    leaf: u64,
    light: u64,
    children: [u32; 64],
    padding: [u8; 8],
}

#[derive(Debug, Clone)]
struct Contree {
    center: f32,
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
