use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos3Norm3Tan4Tex2 {
    pub pos: [f32; 3],
    pub normal: [f32; 3],
    pub tangent: [f32; 4],
    pub tex: [f32; 2],
}

impl Pos3Norm3Tan4Tex2 {
    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self {
            pos: [x, y, z],
            normal: [0.0, 0.0, 0.0],
            tangent: [0.0, 0.0, 0.0, 0.0],
            tex: [0.0, 0.0],
        }
    }

    pub fn from_pos_tex(x: f32, y: f32, z: f32, u: f32, v: f32) -> Self {
        Self {
            pos: [x, y, z],
            normal: [0.0, 0.0, 0.0],
            tangent: [0.0, 0.0, 0.0, 0.0],
            tex: [u, v],
        }
    }
}
unsafe impl Zeroable for Pos3Norm3Tan4Tex2 {}
unsafe impl Pod for Pos3Norm3Tan4Tex2 {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pos4Norm4Tan4 {
    pub pos: [f32; 4],
    pub normal: [f32; 4],
    pub tangent: [f32; 4],
}

unsafe impl Zeroable for Pos4Norm4Tan4 {}
unsafe impl Pod for Pos4Norm4Tan4 {}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PushDataModelTarget {
    pub model: [f32; 16],
}
unsafe impl Zeroable for PushDataModelTarget {}
unsafe impl Pod for PushDataModelTarget {}
