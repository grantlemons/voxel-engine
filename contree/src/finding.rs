use std::iter::FusedIterator;

use glam::Vec3;

use super::{Addr, ChildIndex, Contree, util::*};

pub struct FindResult {
    pub material: Option<u8>,
    pub leaf_address: Option<Addr>,
    pub traversal_iter: Box<dyn FusedIterator<Item = ChildIndex>>,
    pub parent_addrs: Vec<Addr>,
    /// Distance from face to face
    pub node_size: u32,
}

impl Contree {
    pub fn find(&self, pos: Vec3, given_parent_addrs: &[Addr]) -> FindResult {
        let mut traversal_iter =
            morton_iter(morton_code(self.normalize(pos)), self.size.ilog(4) as u8)
                .skip(given_parent_addrs.len())
                .peekable();

        let mut parent_addrs = given_parent_addrs.to_vec();
        let mut current = if let Some(root) = self.root {
            parent_addrs.push(root);
            self.inners[root as usize]
        } else {
            dbg!("No root!");
            return FindResult {
                material: None,
                leaf_address: None,
                traversal_iter: Box::new(traversal_iter),
                parent_addrs,
                node_size: self.size,
            };
        };

        loop {
            // should not panic unless more than one element is popped per iteration
            let index = traversal_iter.peek().expect("Traversal iter empty!");
            let child_addr = current.children[*index as usize] as Addr;

            let child_exists = (current.contains >> index) & 1 == 1;
            let child_leaf = (current.leaf >> index) & 1 == 1;

            if child_exists && child_leaf {
                // leaf node contains this coordinate
                // this does not mean that something exists at this coordinate
                traversal_iter.next();

                // check if something exists at pos
                let leaf = self.leaves[child_addr as usize];
                let index = traversal_iter.peek().expect("Traversal iter empty!");
                let contains = leaf.contains & (1 << index) != 0;

                let node_size = if contains {
                    self.size >> ((parent_addrs.len() as u32 + 1) * 2)
                } else {
                    self.size >> ((parent_addrs.len() as u32) * 2)
                };

                return FindResult {
                    material: if contains {
                        Some(leaf.children[*index as usize])
                    } else {
                        None
                    },
                    leaf_address: Some(child_addr),
                    traversal_iter: Box::new(traversal_iter),
                    node_size,
                    parent_addrs,
                };
            } else if child_exists {
                traversal_iter.next().expect("Traversal iter empty!");
                parent_addrs.push(child_addr);
                current = self.inners[child_addr as usize];
            } else {
                return FindResult {
                    material: None,
                    leaf_address: None,
                    traversal_iter: Box::new(traversal_iter),
                    node_size: self.size >> ((parent_addrs.len() as u32 - 1) * 2),
                    parent_addrs,
                };
            }
        }
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

        assert_eq!(contree.find(p, &[]).node_size, 1);
        assert_eq!(contree.find(Vec3::splat(1.), &[]).node_size, 4);
        assert_eq!(contree.find(Vec3::splat(8.), &[]).node_size, 16);
        assert_eq!(contree.find(Vec3::splat(30.), &[]).node_size, 64);
    }

    #[test]
    fn traverse_empty() {
        let contree = Contree::default();
        let FindResult {
            material,
            leaf_address,
            traversal_iter,
            parent_addrs,
            node_size,
        } = contree.find(Vec3::new(5., 8., 9.), &[]);

        assert!(material.is_none());
        assert!(leaf_address.is_none());
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[36, 5]);
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
            traversal_iter,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        assert_eq!(material, Some(10));
        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0]);
        assert_eq!(node_size, 1);
    }

    #[test]
    fn root_as_parent() {
        let contree = create_contree(16, Vec3::splat(0.));

        assert_eq!(contree.find(Vec3::splat(-1.), &[]).parent_addrs, &[0]);
        assert_eq!(
            contree
                .find(Vec3::splat(-1.), &[])
                .traversal_iter
                .collect::<Vec<_>>(),
            &[7, 63]
        );
        assert_eq!(contree.find(Vec3::splat(32.), &[]).parent_addrs, &[0]);
        assert_eq!(
            contree.find(Vec3::splat(0.), &[]).parent_addrs.first(),
            Some(&0)
        );
    }

    #[test]
    fn insert_traverse_tiny() {
        let p = Vec3::splat(0.);
        let contree = create_contree(64, p);

        let FindResult {
            material,
            leaf_address,
            traversal_iter,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        assert_eq!(material, Some(10));
        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0, 1]);
        assert_eq!(node_size, 1);
    }

    #[test]
    fn find_out_of_bounds() {
        let contree = Contree::default();
        let p = Vec3::splat(contree.size as f32);
        let FindResult {
            leaf_address,
            parent_addrs,
            node_size,
            ..
        } = contree.find(p, &[]);

        assert_eq!(leaf_address, None);
        assert_eq!(parent_addrs.as_slice(), &[0]);
        assert_eq!(node_size, contree.size);
    }
}
