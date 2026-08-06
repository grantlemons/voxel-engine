use glam::Vec3;

use super::{Addr, ChildIndex, Contree, finding::FindResult, util::*};

impl Contree {
    /// Grow upward until the position is in bounds
    fn grow_to_accomodate(&mut self, pos: Vec3) {
        // ensures there is a root
        if self.root.is_none() {
            self.root = Some(self.create_root_node());
            self.center_offset = pos;
        }

        while !self.in_bounds(pos) {
            let new_root = self.create_root_node();

            // TODO: Find a better way to grow
            let new_center = ((pos - self.center_offset) / self.size as f32)
                .round()
                .clamp(-Vec3::splat(3.), Vec3::splat(3.)) // clamp insures the current tree is enclosed
                * self.size as f32;
            let old_root_coords = self.center_offset;
            self.size *= 4;
            self.center_offset = new_center;

            // let mut next_morton_index = MAX_MORTON_INDEX + 1 - (self.size.ilog(4) as u8);
            let old_root_new_index = morton_index(
                morton_code(self.normalize(old_root_coords)),
                MAX_MORTON_INDEX,
            )
            .unwrap();

            // set current node as child of new node
            self.inners[new_root as usize].children[old_root_new_index as usize] =
                self.root.unwrap();
            self.binding
                .write_inner(new_root, &[self.inners[new_root as usize]]);

            self.root = Some(new_root);
        }
    }
    pub fn insert(&mut self, pos: Vec3, material: u8) -> Option<FindResult> {
        self.grow_to_accomodate(pos);

        let FindResult {
            leaf_address,
            traversal_state: traversal_iter,
            mut parent_address,
            ..
        } = self.find(pos)?;

        let (code, mut next_morton_index) = traversal_iter;
        match leaf_address {
            Some(leaf_addr) => {
                let leaf = self
                    .leaves
                    .get_mut(leaf_addr as usize)
                    .expect("Leaf node does not exist!");

                let child_index = morton_index(code, next_morton_index)
                    .expect("Traversal iter should not be empty!");
                next_morton_index += 1;

                leaf.children[child_index as usize] = material;
                leaf.contains |= 1 << child_index;
                self.binding.write_leaf(leaf_addr, &[*leaf]);
            }
            None => {
                let (leaf_addr, child_index) =
                    self.add_parents(traversal_iter, &mut parent_address);
                next_morton_index = MAX_MORTON_INDEX + 1;

                let leaf = self
                    .leaves
                    .get_mut(leaf_addr as usize)
                    .expect("Leaf node does not exist!");

                leaf.children[child_index as usize] = material;
                leaf.contains |= 1 << child_index;
                self.binding.write_leaf(leaf_addr, &[*leaf]);
            }
        }
        Some(FindResult {
            material: Some(material),
            leaf_address,
            parent_address,
            traversal_state: (code, next_morton_index), // this might need to be the prev traversal iter
            node_size: 1,
        })
    }

    fn add_parents(
        &mut self,
        traversal_iter: (u64, u8),
        parent_address: &mut Addr,
    ) -> (Addr, ChildIndex) {
        let mut leaf_addr = 0;
        for i in traversal_iter.1..=MAX_MORTON_INDEX {
            let parent: Addr = *parent_address;
            let child_index = morton_index(traversal_iter.0, i).unwrap();
            match i {
                MAX_MORTON_INDEX => return (leaf_addr, child_index),
                x if x == MAX_MORTON_INDEX - 1 => {
                    leaf_addr = self.create_leaf_node(parent, child_index)
                }
                _ => *parent_address = self.create_inner_node(parent, child_index),
            }
        }
        unreachable!("Never reached bottom of traversal stack!")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ContreeInner;

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
            root: Some(0),
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

        assert_eq!(contree.root, Some(0));
        assert_eq!(contree.size, 4_u32.pow(3));
    }

    #[test]
    fn insert_impacts_parent_contains() {
        let p = Vec3::ZERO;
        let contree = create_contree(16, p);

        let FindResult { parent_address, .. } = contree.find(p).unwrap();
        let bitflag = contree.inners[parent_address as usize].leaf;
        let leaf_index: ChildIndex = morton_index(
            morton_code(contree.normalize(p)),
            MAX_MORTON_INDEX + 1 - (contree.size.ilog2() as u8 / 2),
        )
        .unwrap();

        assert!((bitflag >> leaf_index & 1) == 1);
    }

    #[test]
    fn insert_impacts_parent_leaf() {
        let p = Vec3::ZERO;
        let contree = create_contree(16, p);

        let FindResult { parent_address, .. } = contree.find(p).unwrap();
        let bitflag = contree.inners[parent_address as usize].leaf;
        let leaf_index: ChildIndex = morton_index(
            morton_code(contree.normalize(p)),
            MAX_MORTON_INDEX + 1 - (contree.size.ilog2() as u8 / 2),
        )
        .unwrap();

        assert!((bitflag >> leaf_index & 1) == 1);
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
