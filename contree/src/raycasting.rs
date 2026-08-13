use super::{Contree, finding::FindResult, util::*};
use glam::Vec3;

impl Contree {
    pub fn raycast(&self, pos: Vec3, dir: Vec3) -> Option<Vec3> {
        let norm_dir = dir.normalize();
        let inv_norm_dir = norm_dir.recip();
        let mut p = pos + 0.5 - self.center_offset;
        let dir_pos = dir
            .map(|v| if v == 0. { 0. } else { v.signum() })
            .max(Vec3::ZERO);

        if !self.in_bounds(p - 0.5 + self.center_offset) {
            let boundary = Vec3::splat((self.size / 2) as f32) * p.signum();
            p += ((boundary - p) * inv_norm_dir)
                .to_array()
                .into_iter()
                .filter(|x| x.is_normal())
                .reduce(f32::max)?
                * norm_dir;
        }

        let mut find_p = p + (norm_dir * 0.00001);
        let mut i = 0;
        while self.in_bounds(find_p - 0.5 + self.center_offset) && i < 50 {
            let FindResult {
                leaf_address,
                parent_address,
                traversal_state: (code, next_morton_index),
                depth,
                ..
            } = self.find(find_p - 0.5 + self.center_offset)?;

            // break if hit solid
            if let Some(laddr) = leaf_address
                && let Some(cidx) = morton_index(code, next_morton_index)
                && self.leaves[laddr as usize].contains & (0b1 << cidx) != 0
                && self.leaves[laddr as usize].children[cidx as usize] != 0
            {
                return Some(p - 0.5 + self.center_offset);
            }

            // When moving in a node, unless you know it has no children, you can only move 1/4 at a time
            let child_size =
                if leaf_address.is_some() || self.inners[parent_address as usize].contains != 0 {
                    self.size >> ((depth + 1) << 1)
                } else {
                    self.size >> (depth << 1)
                } as f32;
            let boundary = child_size * ((find_p / child_size).floor() + dir_pos);

            // Maximum t before hitting boundary on each axis
            let max_t = (boundary - p) * inv_norm_dir;

            // WARN: May have platform-dependent behavior
            p += max_t.abs().min_element() * norm_dir;

            find_p = p + (norm_dir * 0.00001);
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
            contree.raycast(Vec3::new(100., 50., 0.), -Vec3::new(2., 1., 0.)),
            Some(Vec3::new(0.5, 0.25, 0.))
        );

        assert_eq!(
            contree.raycast(-Vec3::new(100., 50., 0.), Vec3::new(2., 1., 0.)),
            Some(-Vec3::new(0.5, 0.25, 0.))
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
