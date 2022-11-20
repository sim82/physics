use bevy::{
    math::Vec3Swizzles,
    prelude::{Vec2, Vec3},
};

use super::Vertex;

pub struct Texgen {
    pub translate: Vec2,
    pub scale: Vec2,
    pub rotate: f32,
    pub d_rotate: f32,
    pub shift: Vec2,
}
impl Default for Texgen {
    fn default() -> Self {
        Self {
            translate: Vec2::ZERO,
            scale: Vec2::ONE,
            rotate: 0.0,
            d_rotate: 0.0,
            shift: Vec2::ZERO,
        }
    }
}

impl Texgen {
    pub fn project_tc_for_pos(&self, pos: Vec3, normal: Vec3) -> Vec2 {
        let (s, c) = (-self.rotate).to_radians().sin_cos();
        let c2 = (-self.d_rotate).to_radians().cos();
        let t = (-self.d_rotate).to_radians().tan();

        let tc_trans = pos.yz() - self.translate; // TODO: get major axis from normal
        let tc_rot = Vec2::new(
            tc_trans.x * c - tc_trans.y * s,
            tc_trans.x * s + tc_trans.y * c,
        );
        let tc_shear = Vec2::new(tc_rot.x + tc_rot.y * t, tc_rot.y / c2);
        let tc_scale = tc_shear / self.scale;

        tc_scale - self.shift
    }
}
