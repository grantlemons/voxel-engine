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

#[derive(Debug, Clone, Default)]
struct Contree {
    root: usize,
    /// Distance from center to face
    size: u32,
    inners: Vec<ContreeInner>,
    leaves: Vec<ContreeLeaf>,
    inner_tombstones: Vec<usize>,
    leaf_tombstones: Vec<usize>,
}

fn morton_code(p: UVec3) -> u64 {
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
    (interleave(p.x) << 2) | (interleave(p.y) << 1) | interleave(p.z)
}

impl Contree {
    fn normalize(&self, p: Vec3) -> Vec3 {
        p + Vec3::splat(self.size as f32)
    }

    fn svo_abs(v: i32) -> i32 {
        if v < 0 { -v - 1 } else { v }
    }

    fn in_bounds(self, p: IVec3) -> bool {
        (p.map(Self::svo_abs).max_element() as u32) < self.size
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

    fn find(&self, normalized_pos: Vec3) -> (Option<usize>, usize, Vec<usize>) {
        fn to_base_64(code: u64) -> Vec<usize> {
            let mut digits = Vec::new();
            let mut n = code;
            if n == 0 {
                digits.push(0);
                return digits;
            }
            while n != 0 {
                digits.push((n % 64) as usize);
                n /= 64
            }
            digits.reverse();
            digits
        }

        let code = morton_code(normalized_pos.as_uvec3());
        let traversal_indexes = to_base_64(code);

        let mut parent_indexes = Vec::new();
        if self.inners.is_empty() {
            return (None, *traversal_indexes.first().unwrap(), parent_indexes);
        }

        parent_indexes.push(self.root);
        let mut current = self.inners[self.root];
        for i in 0..(traversal_indexes.len()) {
            let index = traversal_indexes[i];
            let child_addr = current.children[index] as usize;

            let child_exists = current.contains & (1 << index) != 0;
            let child_leaf = current.leaf & (1 << index) != 0;

            if child_exists && child_leaf {
                return (Some(child_addr), traversal_indexes[i + 1], parent_indexes);
            }
            if child_exists {
                parent_indexes.push(child_addr);
                current = self.inners[child_addr];
            } else {
                return (None, index, parent_indexes);
            }
        }

        (None, *traversal_indexes.first().unwrap(), parent_indexes)
    }
}

#[cfg(test)]
mod tests {
    use glam::{UVec3, Vec3};

    use crate::contree::{Contree, ContreeInner, ContreeLeaf, morton_code};

    #[test]
    fn morton_code_example() {
        let code = morton_code(UVec3::new(5, 8, 9));
        assert_eq!(code, 0b011100000101);
    }

    #[test]
    fn traverse_empty() {
        let p = Vec3::new(5., 8., 9.);
        let contree = Contree::default();
        let (child_addr, _next_index, path) = contree.find(contree.normalize(p));
        assert!(child_addr.is_none());
        assert!(path.is_empty());
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

        let (child_addr, next_index, path) = contree.find(contree.normalize(p));
        assert_eq!(child_addr, Some(0));
        assert_eq!(next_index, 0);
        assert!(matches!(path.as_slice(), &[0]));
    }
}
