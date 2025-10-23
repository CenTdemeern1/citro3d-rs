use std::{cell::RefMut, marker::PhantomData, ops::Deref};

pub use citro3d::render::RenderTarget;
use ctru::services::gfx::Screen;

use crate::{Error, Result, shapes::Shape};

/// A color in RGBA format. The color is stored as a 32-bit integer
#[derive(Debug, Clone, Copy)]
pub struct Color {
    pub inner: u32,
}

impl Color {
    /// Create a new color with the given RGB values. Alpha is set to 255 (fully opaque).
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self::new_with_alpha(r, g, b, 255)
    }

    /// Create a new color with the given RGBA values.
    pub fn new_with_alpha(r: u8, g: u8, b: u8, a: u8) -> Self {
        let inner = r as u32 | (g as u32) << 8 | (b as u32) << 16 | (a as u32) << 24;
        Self { inner }
    }
}

impl Into<Color> for u32 {
    fn into(self) -> Color {
        Color { inner: self }
    }
}

impl From<Color> for u32 {
    fn from(color: Color) -> u32 {
        color.inner
    }
}

pub trait TargetExt {
    /// Clears the screen to a specific [Color]
    fn clear_with_color(&mut self, color: Color);

    /// Renders a 2d shape to the [Target]
    fn render_2d_shape(&mut self, shape: &impl Shape);
}

impl<'screen> TargetExt for RenderTarget<'screen> {
    fn clear_with_color(&mut self, color: Color) {
        unsafe {
            citro2d_sys::C2D_TargetClear(self.as_raw(), color.inner);
        }
    }

    /// Renders a 2d shape to the [Target]
    fn render_2d_shape(&mut self, shape: &impl Shape) {
        shape.render(self);
    }
}

pub struct ScreenTarget<'screen>(RenderTarget<'screen>);

impl<'screen> ScreenTarget<'screen> {
    //
    pub unsafe fn inner_mut(&mut self) -> &'screen mut RenderTarget {
        &mut self.0
    }
}
