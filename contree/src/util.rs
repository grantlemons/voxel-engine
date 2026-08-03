use glam::{U64Vec3, UVec3, Vec3};

use crate::ChildIndex;

use super::Contree;

pub fn morton_code(norm_p: UVec3) -> u64 {
    let mut res = (norm_p & UVec3::splat(0x1fffff)).as_u64vec3();
    res = (res | res << 32) & U64Vec3::splat(0x1f00000000ffff);
    res = (res | res << 16) & U64Vec3::splat(0x1f0000ff0000ff);
    res = (res | res << 8) & U64Vec3::splat(0x100f00f00f00f00f);
    res = (res | res << 4) & U64Vec3::splat(0x10c30c30c30c30c3);
    res = (res | res << 2) & U64Vec3::splat(0x1249249249249249);

    (res.x << 2) | (res.y << 1) | res.z
}

pub const MAX_MORTON_INDEX: u8 = 8;
pub fn morton_index(code: u64, index: u8) -> Option<ChildIndex> {
    if index > MAX_MORTON_INDEX {
        None
    } else {
        Some(((code >> (48 - 6 * index)) & 0b111111) as ChildIndex)
    }
}

pub fn round_in_dir(x: Vec3, dir: Vec3) -> Vec3 {
    let neg = Vec3::new(x.x - 0.5, x.y - 0.5, x.z - 0.5).ceil();
    let pos = Vec3::new(x.x + 0.5, x.y + 0.5, x.z + 0.5).floor();
    Vec3::select(dir.cmplt(Vec3::ZERO), neg, pos)
}

impl Contree {
    pub(super) fn normalize(&self, p: Vec3) -> UVec3 {
        (p - self.center_offset + (self.size as f32 / 2.))
            .round()
            .as_uvec3()
    }

    pub(super) fn in_bounds(&self, p: Vec3) -> bool {
        fn svo_abs(v: f32) -> f32 {
            if v < 0. { -v - 1. } else { v }
        }
        (p - self.center_offset)
            .map(svo_abs)
            .round()
            .as_uvec3()
            .max_element()
            < self.size / 2
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use glam::UVec3;

    #[test]
    fn morton_code_example() {
        let code = morton_code(UVec3::new(5, 8, 9));
        assert_eq!(code, 0b011100000101);
    }

    #[test]
    fn morton_code_zero() {
        let code = morton_code(UVec3::new(0, 0, 0));

        let mut next_morton_index = MAX_MORTON_INDEX + 1 - 3;
        let traversal_iter = std::iter::from_fn(|| {
            let res = morton_index(code, next_morton_index);
            next_morton_index += 1;
            res
        });

        assert_eq!(code, 0);
        assert_eq!(traversal_iter.collect::<Vec<_>>(), &[0, 0, 0]);
    }

    #[test]
    fn round_down() {
        assert_eq!(
            round_in_dir(Vec3::splat(0.5), Vec3::splat(-1.)),
            Vec3::splat(0.)
        )
    }

    #[test]
    fn round_up() {
        assert_eq!(
            round_in_dir(Vec3::splat(0.5), Vec3::splat(1.)),
            Vec3::splat(1.)
        )
    }

    #[test]
    fn contains_skews_negative() {
        let contree = Contree::default();

        assert!(contree.in_bounds(Vec3::splat(-8.)));
        assert!(!contree.in_bounds(Vec3::splat(8.)));
    }
}
