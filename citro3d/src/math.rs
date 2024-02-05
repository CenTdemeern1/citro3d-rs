//! Safe wrappers for working with matrix and vector types provided by `citro3d`.

// TODO: bench FFI calls into `inline statics` generated by bindgen, vs
// reimplementing some of those calls. Many of them are pretty trivial impls

mod fvec;
mod matrix;
mod ops;
mod projection;

pub use fvec::{FVec, FVec3, FVec4};
pub use matrix::Matrix4;
pub use projection::{
    AspectRatio, ClipPlanes, CoordinateOrientation, Orthographic, Perspective, Projection,
    ScreenOrientation, StereoDisplacement,
};

/// A 4-vector of `u8`s.
///
/// # Layout
/// Uses the PICA layout of WZYX
#[doc(alias = "C3D_IVec")]
#[repr(transparent)]
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct IVec(citro3d_sys::C3D_IVec);

impl IVec {
    pub fn new(x: u8, y: u8, z: u8, w: u8) -> Self {
        Self(unsafe { citro3d_sys::IVec_Pack(x, y, z, w) })
    }
    pub fn as_raw(&self) -> &citro3d_sys::C3D_IVec {
        &self.0
    }
    pub fn x(self) -> u8 {
        self.0 as u8
    }
    pub fn y(self) -> u8 {
        (self.0 >> 8) as u8
    }
    pub fn z(self) -> u8 {
        (self.0 >> 16) as u8
    }
    pub fn w(self) -> u8 {
        (self.0 >> 24) as u8
    }
}

/// A quaternion, internally represented the same way as [`FVec`].
#[doc(alias = "C3D_FQuat")]
pub struct FQuat(citro3d_sys::C3D_FQuat);

#[cfg(test)]
mod tests {
    use super::IVec;

    #[test]
    fn ivec_getters_work() {
        let iv = IVec::new(1, 2, 3, 4);
        assert_eq!(iv.x(), 1);
        assert_eq!(iv.y(), 2);
        assert_eq!(iv.z(), 3);
        assert_eq!(iv.w(), 4);
    }
}
