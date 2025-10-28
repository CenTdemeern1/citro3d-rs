//! Safe bindings to citro2d drawables
use std::ops::{ControlFlow, FromResidual, Try};

use citro3d::render::RenderTarget;

use crate::{Point, Size, render::Color};

#[repr(u8)] // It's essentially just a bool
pub enum DrawableResult {
    Success = 1,
    Failure = 0,
}

impl Try for DrawableResult {
    type Output = ();
    type Residual = DrawableResult;

    fn from_output(_output: Self::Output) -> Self {
        DrawableResult::Success
    }

    fn branch(self) -> ControlFlow<Self::Residual, Self::Output> {
        use ControlFlow::*;
        use DrawableResult::*;
        match self {
            Success => Continue(()),
            Failure => Break(Failure),
        }
    }
}

impl FromResidual for DrawableResult {
    fn from_residual(residual: <Self as Try>::Residual) -> Self {
        residual
    }
}

impl From<bool> for DrawableResult {
    fn from(value: bool) -> Self {
        use DrawableResult::*;
        if value { Success } else { Failure }
    }
}

/// Holds information for rendering multi colored shapes
/// most shapes have a 'solid'
pub struct MultiColor {
    pub top_left: Color,
    pub top_right: Color,
    pub bottom_left: Color,
    pub bottom_right: Color,
}

/// A trait for renderable items.
///
/// You may implement this trait yourself to create composite/custom drawables.
pub trait Drawable {
    //TODO possibly return Option<self>.
    fn render(&self, target: &mut RenderTarget<'_>) -> DrawableResult;
}

/// Holds information for rendering a C2D_DrawRectangle
pub struct Rectangle {
    pub point: Point,
    pub size: Size,
    pub multi_color: MultiColor,
}

impl Drawable for Rectangle {
    /// Draws a multi color rectangle
    #[doc(alias = "C2D_DrawRectangle")]
    fn render(&self, _target: &mut RenderTarget<'_>) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawRectangle(
                self.point.x,
                self.point.y,
                self.point.z,
                self.size.width,
                self.size.height,
                self.multi_color.top_left.into(),
                self.multi_color.top_right.into(),
                self.multi_color.bottom_left.into(),
                self.multi_color.bottom_right.into(),
            )
        }
        .into()
    }
}

/// Holds the information needed to draw a solid color Rectangle
pub struct RectangleSolid {
    pub point: Point,
    pub size: Size,
    pub color: Color,
}

impl Drawable for RectangleSolid {
    /// Draws a single colored Rectangle
    #[doc(alias = "C2D_DrawRectSolid")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawRectSolid(
                self.point.x,
                self.point.y,
                self.point.z,
                self.size.width,
                self.size.height,
                self.color.into(),
            )
        }
        .into()
    }
}

/// Holds the information needed to draw a solid color Triangle
pub struct Triangle {
    pub top: Point,
    pub top_color: Color,
    pub left: Point,
    pub left_color: Color,
    pub right: Point,
    pub right_color: Color,
    pub depth: f32,
}

impl Drawable for Triangle {
    /// Draws a multi color Triangle
    #[doc(alias = "C2D_DrawTriangle")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawTriangle(
                self.top.x,
                self.top.y,
                self.top_color.into(),
                self.left.x,
                self.left.y,
                self.left_color.into(),
                self.right.x,
                self.right.y,
                self.right_color.into(),
                self.depth,
            )
        }
        .into()
    }
}

/// Holds the information needed to draw a Ellipse
pub struct Ellipse {
    pub point: Point,
    pub size: Size,
    pub multi_color: MultiColor,
}

impl Drawable for Ellipse {
    /// Draws a multi color Ellipse
    #[doc(alias = "C2D_DrawEllipse")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawEllipse(
                self.point.x,
                self.point.y,
                self.point.z,
                self.size.width,
                self.size.height,
                self.multi_color.top_left.into(),
                self.multi_color.top_right.into(),
                self.multi_color.bottom_left.into(),
                self.multi_color.bottom_right.into(),
            )
        }
        .into()
    }
}

/// Holds the information needed to draw a solid color Triangle
pub struct EllipseSolid {
    pub point: Point,
    pub size: Size,
    pub color: Color,
}

impl Drawable for EllipseSolid {
    ///Draws a solid color Ellipse
    #[doc(alias = "C2D_DrawEllipseSolid")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawEllipseSolid(
                self.point.x,
                self.point.y,
                self.point.z,
                self.size.width,
                self.size.height,
                self.color.into(),
            )
        }
        .into()
    }
}
/// Holds the information needed to draw a multi colored circle
pub struct Circle {
    pub point: Point,
    pub radius: f32,
    pub multi_color: MultiColor,
}

impl Drawable for Circle {
    /// Draws a multi color Ellipse
    #[doc(alias = "C2D_DrawCircle")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawCircle(
                self.point.x,
                self.point.y,
                self.point.z,
                self.radius,
                self.multi_color.top_left.into(),
                self.multi_color.top_right.into(),
                self.multi_color.bottom_left.into(),
                self.multi_color.bottom_right.into(),
            )
        }
        .into()
    }
}

/// Holds the information needed to draw a solid color Circle
pub struct CircleSolid {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub radius: f32,
    pub color: Color,
}

impl Drawable for CircleSolid {
    /// Renders a solid Circle
    #[doc(alias = "C2D_DrawCircleSolid")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawCircleSolid(self.x, self.y, self.z, self.radius, self.color.into())
        }
        .into()
    }
}

/// Holds the information needed to draw a solid color Circle
pub struct Line {
    pub start: Point,
    pub end: Point,
    pub start_color: Color,
    pub end_color: Color,
    pub thickness: f32,
    pub depth: f32,
}

impl Drawable for Line {
    /// Renders a line
    #[doc(alias = "C2D_DrawLine")]
    fn render(&self, _target: &mut RenderTarget) -> DrawableResult {
        unsafe {
            citro2d_sys::C2D_DrawLine(
                self.start.x,
                self.start.y,
                self.start_color.into(),
                self.end.x,
                self.end.y,
                self.end_color.into(),
                self.thickness,
                self.depth,
            )
        }
        .into()
    }
}
