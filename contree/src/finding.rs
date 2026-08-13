use glam::Vec3;

use super::{Addr, Contree, util::*};

pub struct FindResult {
    pub material: Option<u8>,
    pub leaf_address: Option<Addr>,
    pub parent_address: Addr,
    pub traversal_state: (u64, u8),
    pub depth: u8,
}

impl Contree {
    pub fn find(&self, pos: Vec3) -> Option<FindResult> {
        let code = morton_code(self.normalize(pos));
        let mut next_morton_index = MAX_MORTON_INDEX + 1 - (self.size.ilog2() as u8 / 2);

        let mut depth = 0;
        let mut parent_address: Addr = self.root?;

        while next_morton_index < MAX_MORTON_INDEX {
            let parent = self.inners[parent_address as usize];
            let index = morton_index(code, next_morton_index).expect("Traversal iter empty!");
            let child_addr = parent.children[index as usize] as Addr;

            let child_exists = (parent.contains >> index) & 1 == 1;
            let child_leaf = (parent.leaf >> index) & 1 == 1;

            if child_exists {
                depth += 1;
                next_morton_index += 1;

                // leaf node contains this coordinate
                // this does not mean that something exists at this coordinate
                if child_leaf {
                    let leaf = self.leaves[child_addr as usize];
                    let index =
                        morton_index(code, next_morton_index).expect("Traversal iter empty!");
                    let contains = (leaf.contains >> index) & 1 == 1;

                    return Some(FindResult {
                        material: contains.then(|| leaf.children[index as usize]),
                        leaf_address: Some(child_addr),
                        parent_address,
                        traversal_state: (code, next_morton_index),
                        depth: if contains { depth + 1 } else { depth },
                    });
                }

                parent_address = child_addr;
            } else {
                return Some(FindResult {
                    material: None,
                    leaf_address: None,
                    parent_address,
                    traversal_state: (code, next_morton_index),
                    depth,
                });
            }
        }
        unreachable!();
    }
}

#[cfg(test)]
mod tests {
    use crate::{ContreeInner, ContreeLeaf};

    use super::*;

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
    fn node_sizing() {
        let p = Vec3::splat(0.);
        let contree = create_contree(64, p);

        let size = |x| contree.size >> (x * 2);

        assert_eq!(size(contree.find(p).unwrap().depth), 1);
        assert_eq!(size(contree.find(Vec3::splat(1.)).unwrap().depth), 4);
        assert_eq!(size(contree.find(Vec3::splat(8.)).unwrap().depth), 16);
        assert_eq!(size(contree.find(Vec3::splat(30.)).unwrap().depth), 64);
    }

    #[test]
    fn traverse_empty() {
        let contree = Contree::default();
        let FindResult {
            material,
            leaf_address,
            traversal_state: (code, mut next_morton_index),
            parent_address,
            depth,
        } = contree.find(Vec3::new(5., 8., 9.)).unwrap();

        let traversal_iter = std::iter::from_fn(|| {
            let res = morton_index(code, next_morton_index);
            next_morton_index += 1;
            res
        });

        assert!(material.is_none());
        assert!(leaf_address.is_none());
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[36, 5]);
        assert_eq!(parent_address, 0);
        assert_eq!(depth, 0);
    }

    #[test]
    fn traverse_tiny() {
        let p = Vec3::new(0., 0., 0.);

        let mut inner_children = [0; 64];
        let mut leaf_children = [0; 64];
        inner_children[56] = 0;
        leaf_children[0] = 10;
        let contree = Contree {
            root: Some(0),
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
            material,
            leaf_address,
            traversal_state: (code, mut next_morton_index),
            parent_address,
            depth,
        } = contree.find(p).unwrap();

        let traversal_iter = std::iter::from_fn(|| {
            let res = morton_index(code, next_morton_index);
            next_morton_index += 1;
            res
        });

        assert_eq!(material, Some(10));
        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[0]);
        assert_eq!(parent_address, 0);
        assert_eq!(depth, 2);
    }

    #[test]
    fn root_as_parent() {
        let p = Vec3::ZERO;
        let contree = create_contree(16, p);

        let (code, mut next_morton_index) = contree.find(Vec3::splat(-1.)).unwrap().traversal_state;
        let traversal_iter = std::iter::from_fn(|| {
            let res = morton_index(code, next_morton_index);
            next_morton_index += 1;
            res
        });

        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[7, 63]);
        assert_eq!(contree.find(p).unwrap().parent_address, 0);
    }

    #[test]
    fn insert_traverse_tiny() {
        let p = Vec3::splat(0.);
        let contree = create_contree(64, p);

        let FindResult {
            material,
            leaf_address,
            traversal_state: (code, mut next_morton_index),
            parent_address,
            depth,
        } = contree.find(p).unwrap();

        let traversal_iter = std::iter::from_fn(|| {
            let res = morton_index(code, next_morton_index);
            next_morton_index += 1;
            res
        });

        assert_eq!(material, Some(10));
        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[0]);
        assert_eq!(parent_address, 1);
        assert_eq!(depth, 3);
    }

    #[test]
    fn find_out_of_bounds() {
        let contree = Contree::default();
        let p = Vec3::splat(contree.size as f32);
        let FindResult {
            leaf_address,
            parent_address,
            depth,
            ..
        } = contree.find(p).unwrap();

        assert_eq!(leaf_address, None);
        assert_eq!(parent_address, 0);
        assert_eq!(depth, 0);
    }
}
