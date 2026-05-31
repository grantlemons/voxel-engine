#[allow(clippy::module_inception)]
#[cfg(test)]
mod tests {
    use crate::contree::{Contree, ContreeInner, ContreeLeaf, FindResult};
    use glam::Vec3;

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
    fn traverse_empty() {
        let contree = Contree::default();
        let FindResult {
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size,
        } = contree.find(Vec3::new(5., 8., 9.), &[]);

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
    fn root_as_parent() {
        let contree = create_contree(16, Vec3::splat(0.));

        assert_eq!(contree.find(Vec3::splat(-1.), &[]).parent_addrs, &[0]);
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
            leaf_address,
            traversal_stack,
            parent_addrs,
            node_size,
        } = contree.find(p, &[]);

        assert_eq!(leaf_address, Some(0));
        assert_eq!(traversal_stack.as_slice(), &[0]);
        assert_eq!(parent_addrs.as_slice(), &[0, 1]);
        assert_eq!(node_size, 1);
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
        assert!(contree.in_bounds(Vec3::splat(-15.)));
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

        assert!(contree.in_bounds(Vec3::splat(-7.)));
    }

    #[test]
    fn raycast_in_bounds() {
        let contree = create_contree(64, Vec3::splat(-1.));

        assert!(contree.raycast(Vec3::splat(0.), Vec3::splat(1.)).is_none());
        assert_eq!(
            contree.raycast(Vec3::splat(0.), Vec3::splat(-1.)),
            Some(Vec3::splat(-0.5))
        );
        assert!(
            contree
                .raycast(Vec3::new(0., -30., 0.), Vec3::new(0., -1., 0.))
                .is_none(),
        );
        assert!(
            contree
                .raycast(Vec3::new(0., -30., 0.), Vec3::new(0., 1., 0.))
                .is_none(),
        );
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

    #[test]
    fn raycast_out_of_bounds() {
        let contree = create_contree(64, Vec3::splat(0.));

        assert_eq!(
            contree.raycast(Vec3::new(100., 50., 0.), Vec3::new(-2., -1., 0.)),
            Some(Vec3::new(0.5, 0.25, 0.))
        );
    }

    #[test]
    fn raycast_out_of_bounds_wrong_dir() {
        let contree = create_contree(64, Vec3::splat(0.));

        assert!(
            contree
                .raycast(Vec3::new(-100., 0., 0.), Vec3::new(-1., 0., 0.))
                .is_none()
        );
    }
}
