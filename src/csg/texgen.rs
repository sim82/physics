use bevy::{
    math::Vec3Swizzles,
    prelude::{Vec2, Vec3},
};

pub enum MajorAxis {
    None,
    Xpos,
    Xneg,
    Ypos,
    Yneg,
    Zpos,
    Zneg,
}

impl MajorAxis {
    pub fn project(&self, v: Vec3) -> Vec2 {
        // v.y = -v.y;
        match self {
            MajorAxis::None => Vec2::ZERO,
            MajorAxis::Xpos | MajorAxis::Xneg => v.zy(),
            MajorAxis::Ypos | MajorAxis::Yneg => v.xz(),
            MajorAxis::Zpos | MajorAxis::Zneg => v.xy(),
        }
    }
}

impl From<Vec3> for MajorAxis {
    fn from(v: Vec3) -> Self {
        let v_abs = v.abs();

        if v_abs.x >= v_abs.y && v_abs.x >= v_abs.z {
            if v.x >= 0.0 {
                MajorAxis::Xpos
            } else {
                MajorAxis::Xneg
            }
        } else if v_abs.y >= v_abs.x && v_abs.y >= v_abs.z {
            if v.y >= 0.0 {
                MajorAxis::Ypos
            } else {
                MajorAxis::Yneg
            }
        } else if v_abs.z >= v_abs.x && v_abs.z >= v_abs.y {
            if v.z >= 0.0 {
                MajorAxis::Zpos
            } else {
                MajorAxis::Zneg
            }
        } else {
            MajorAxis::None
        }
    }
}

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

        let tc = MajorAxis::from(normal).project(pos);
        let tc_trans = tc - self.translate;
        let tc_rot = Vec2::new(
            tc_trans.x * c - tc_trans.y * s,
            tc_trans.x * s + tc_trans.y * c,
        );
        let tc_shear = Vec2::new(tc_rot.x + tc_rot.y * t, tc_rot.y / c2);
        let tc_scale = tc_shear / self.scale / 2.0;
        tc_scale - self.shift
    }
}
