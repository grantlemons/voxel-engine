use super::{Addr, Contree, finding::FindResult, util::*};
use glam::Vec3;

impl Contree {
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

    fn raycast_to_bounds(&self, pos: Vec3, dir: Vec3) -> Option<Vec3> {
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

        Some(p)
    }

    pub fn raycast(&self, pos: Vec3, dir: Vec3) -> Option<Vec3> {
        let mut p = self.raycast_to_bounds(pos, dir)?;
        let dir_sign = dir.map(|v| if v == 0. { 0. } else { v.signum() });
        let mut find_p = p + (dir_sign * 0.01);

        let mut i = 0;
        while self.in_bounds(find_p) && i < 50 {
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

            let bspace_p = p + 0.5 - self.center_offset;
            let bspace_boundary =
                child_size * round_in_dir(bspace_p / child_size + dir_sign / 2., dir);
            let pspace_boundary = bspace_boundary - 0.5 + self.center_offset;

            // Maximum t before hitting boundary on each axis
            let norm_dir = dir.normalize();
            let max_t = (pspace_boundary - p) / norm_dir;

            // Minimum element of max_t, ignoring inf, -inf, and NaN values
            let move_distance = max_t
                .abs()
                .to_array()
                .into_iter()
                .filter(|x| x.is_normal())
                .reduce(f32::min)
                .expect("All movement distance options are inf or NaN!");

            p += move_distance * norm_dir; // jump to boundary
            // p[move_axis] = pspace_boundary[move_axis]; // snap to boundary to reduce FPE
            dbg!(p);

            find_p = p + (dir_sign * 0.00001);
            i += 1;
        }
        None
    }
}

#[cfg(test)]
mod tests {
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

    #[test]
    fn raycast_fan() {
        let contree = create_contree(16, Vec3::ZERO);

        let distance = 5.;
        let pos = Vec3::new(distance + 0.5, 0., 0.);

        let mut dir = Vec3::new(-distance, 1., 0.);
        // TODO: Can this be (<=)?
        while dir.y > -1. {
            if dir.y.abs() < 0.5 {
                assert!(contree.raycast(pos, dir).is_some());
            } else {
                assert!(contree.raycast(pos, dir).is_none());
            }
            // round to account for FPE
            dir = ((dir + Vec3::new(0., -0.0005, 0.)) / 0.0005).round() * 0.0005;
        }
    }
}
