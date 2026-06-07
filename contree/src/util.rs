use glam::{UVec3, Vec3};

use super::Contree;

pub fn morton_code(norm_p: glam::UVec3) -> u64 {
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
        x
    }
    (interleave(norm_p.x) << 2) | (interleave(norm_p.y) << 1) | interleave(norm_p.z)
}

pub fn to_base_64(code: u64) -> impl Iterator<Item = usize> {
    let mut n = code;
    let mut first = true;
    std::iter::from_fn(move || {
        if n == 0 {
            if first {
                first = false;
                Some(0usize)
            } else {
                None
            }
        } else {
            first = false;
            let res = Some((n as usize) & 0b111111);
            n >>= 6usize;
            res
        }
    })
}

pub fn round_in_dir(x: Vec3, dir: Vec3) -> Vec3 {
    let res = x
        .to_array()
        .iter()
        .zip(dir.to_array().iter())
        .map(|(&x, &d)| {
            if d < 0. {
                (x - 0.5).ceil()
            } else {
                (x + 0.5).floor()
            }
        })
        .collect::<Vec<f32>>();
    Vec3::from_slice(res.as_slice())
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
        let index = to_base_64(code).collect::<Vec<_>>();
        assert_eq!(code, 0);
        assert_eq!(index, &[0]);
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
