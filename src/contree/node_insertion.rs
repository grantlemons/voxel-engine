use glam::Vec3;

use super::{Addr, ChildIndex, Contree, FindResult, util::*};

impl Contree {
    /// Grow upward until the position is in bounds
    fn grow_to_accomodate(&mut self, pos: Vec3) {
        while !self.in_bounds(pos) {
            let new_root = self.create_root_node();

            // TODO: Find a better way to grow
            let new_center = ((pos - self.center_offset) / self.size as f32)
                .round()
                .clamp(Vec3::splat(-3.), Vec3::splat(3.)) // clamp insures the current tree is enclosed
                * self.size as f32;
            let old_root_coords = self.center_offset;
            self.size *= 4;
            self.center_offset = new_center;
            let old_root_new_index = to_base_64(morton_code(self.normalize(old_root_coords)))
                .last()
                .unwrap();

            // set current node as child of new node
            self.inners[new_root as usize].children[old_root_new_index] = self.root;
            self.gpu
                .write_inner(new_root, &[self.inners[new_root as usize]]);

            self.root = new_root;
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::contree::ContreeInner;

    fn create_contree(size: u32, p: Vec3) -> Contree {
        assert!(size > 4, "The root node cannot be a leaf!");
        let mut contree = Contree {
            size,
            ..Default::default()
        };
        contree.insert(p, 10);
        contree
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
        contree.insert(Vec3::new(1., 0., 0.), 3);
        contree.insert(Vec3::new(-10., 10., 10.), 4);
        contree.insert(Vec3::new(-10., 0., 0.), 5);
        contree.insert(Vec3::new(-10., -10., 0.), 6);

        assert_eq!(contree.root, 0);
        assert_eq!(contree.size, 4_u32.pow(3));
    }

    #[test]
    fn grow_positive() {
        let mut contree = create_contree(16, Vec3::ZERO);

        contree.insert(Vec3::splat(8.), 10);
        assert_eq!(contree.size, 64);
        assert_eq!(contree.center_offset, Vec3::splat(16.));

        assert!(contree.in_bounds(Vec3::splat(32.)));
        assert!(contree.in_bounds(Vec3::splat(-16.)));
        assert!(!contree.in_bounds(Vec3::splat(-32.)));
    }

    #[test]
    fn grow_negative() {
        let mut contree = create_contree(16, Vec3::ZERO);

        contree.insert(Vec3::splat(-9.), 10);
        assert_eq!(contree.size, 64);
        assert_eq!(contree.center_offset, Vec3::splat(-16.));

        assert!(contree.in_bounds(Vec3::splat(-32.)));
        assert!(contree.in_bounds(Vec3::splat(15.)));
        assert!(!contree.in_bounds(Vec3::splat(32.)));
    }

    #[test]
    fn grow_multiple_times() {
        let mut contree = create_contree(16, Vec3::ZERO);

        contree.insert(Vec3::splat(100.), 10);
        assert_eq!(contree.size, 256);
        assert_eq!(contree.center_offset, Vec3::splat(64.));

        assert!(contree.in_bounds(Vec3::splat(-8.)));
    }
}
