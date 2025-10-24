#![feature(custom_test_frameworks)]
#![test_runner(test_runner::run_gdb)]
#![feature(allocator_api)]
#![feature(doc_cfg)]
#![doc(html_root_url = "https://rust3ds.github.io/citro3d-rs/crates")]
#![doc(
    html_favicon_url = "https://user-images.githubusercontent.com/11131775/225929072-2fa1741c-93ae-4b47-9bdf-af70f3d59910.png"
)]
#![doc(
    html_logo_url = "https://user-images.githubusercontent.com/11131775/225929072-2fa1741c-93ae-4b47-9bdf-af70f3d59910.png"
)]

//! Safe Rust bindings to `citro3d`. This crate wraps `citro3d-sys` to provide
//! safer APIs for graphics programs targeting the 3DS.
//!
//! ## Feature flags
#![doc = document_features::document_features!()]

pub mod attrib;
pub mod buffer;
pub mod color;
pub mod error;
pub mod fog;
pub mod light;
pub mod math;
pub mod render;
pub mod shader;
pub mod texenv;
pub mod texture;
pub mod uniform;

use std::cell::{OnceCell, RefMut};
use std::fmt;
use std::pin::Pin;
use std::rc::Rc;

use ctru::services::gfx::Screen;
pub use error::{Error, Result};

use self::buffer::{Index, Indices};
use self::light::LightEnv;
use self::texenv::TexEnv;
use self::uniform::Uniform;
use crate::render::{RenderTarget, ScreenTarget};

pub mod macros {
    //! Helper macros for working with shaders.
    pub use citro3d_macros::*;
}

mod private {
    pub trait Sealed {}
    impl Sealed for u8 {}
    impl Sealed for u16 {}
}

/// The single instance for using `citro3d`. This is the base type that an application
/// should instantiate to use this library.
#[non_exhaustive]
#[must_use]
pub struct Instance {
    texenvs: [OnceCell<TexEnv>; texenv::TEXENV_COUNT],
    queue: Rc<RenderQueue>,
    light_env: Option<Pin<Box<LightEnv>>>,
}

/// Representation of `citro3d`'s internal render queue. This is something that
/// lives in the global context, but it keeps references to resources that are
/// used for rendering, so it's useful for us to have something to represent its
/// lifetime.
struct RenderQueue;

impl fmt::Debug for Instance {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Instance").finish_non_exhaustive()
    }
}

impl Instance {
    /// Initialize the default `citro3d` instance.
    ///
    /// # Errors
    ///
    /// Fails if `citro3d` cannot be initialized.
    pub fn new() -> Result<Self> {
        Self::with_cmdbuf_size(citro3d_sys::C3D_DEFAULT_CMDBUF_SIZE.try_into().unwrap())
    }

    /// Initialize the instance with a specified command buffer size.
    ///
    /// # Errors
    ///
    /// Fails if `citro3d` cannot be initialized.
    #[doc(alias = "C3D_Init")]
    pub fn with_cmdbuf_size(size: usize) -> Result<Self> {
        if unsafe { citro3d_sys::C3D_Init(size) } {
            Ok(Self {
                texenvs: Default::default(),
                queue: Rc::new(RenderQueue),
                light_env: None,
            })
        } else {
            Err(Error::FailedToInitialize)
        }
    }

    /// Create a new render target with the specified size, color format,
    /// and depth format.
    ///
    /// # Errors
    ///
    /// Fails if the target could not be created with the given parameters.
    #[doc(alias = "C3D_RenderTargetCreate")]
    #[doc(alias = "C3D_RenderTargetSetOutput")]
    pub fn create_screen_target<'screen, S: Screen>(
        &self,
        width: usize,
        height: usize,
        screen: RefMut<'screen, S>,
        depth_format: Option<render::DepthFormat>,
    ) -> Result<ScreenTarget<'screen, S>> {
        ScreenTarget::new(width, height, screen, depth_format, Rc::clone(&self.queue))
    }

    pub unsafe fn create_screen_target_from_raw<'screen, S: Screen>(
        &self,
        raw: *mut citro3d_sys::C3D_RenderTarget_tag,
        screen: RefMut<'screen, S>,
    ) -> Result<ScreenTarget<'screen, S>> {
        unsafe { ScreenTarget::from_raw(raw, screen, Rc::clone(&self.queue)) }
    }

    /// Render a frame.
    ///
    /// The passed in function/closure will receive a [RenderInstance]
    /// and [RenderTarget] to grant the ability to render things.
    /// It must also return the RenderTarget afterwards.
    #[doc(alias = "C3D_FrameBegin")]
    #[doc(alias = "C3D_FrameDrawOn")]
    #[doc(alias = "C3D_FrameEnd")]
    pub fn render_to_target<'screen, 'screen2, S, S2, F, T>(
        &mut self,
        screen_target: ScreenTarget<'screen, S>,
        f: F,
    ) -> Result<(ScreenTarget<'screen2, S2>, T)>
    where
        S: Screen + 'screen,
        S2: Screen + 'screen2,
        F: FnOnce(&mut Self, RenderTarget<'screen, S>) -> (RenderTarget<'screen2, S2>, T),
    {
        let render_target = unsafe {
            citro3d_sys::C3D_FrameBegin(
                // TODO: begin + end flags should be configurable
                citro3d_sys::C3D_FRAME_SYNCDRAW,
            );
            self.set_render_target(&screen_target)?;
            screen_target.into_inner()
        };

        let (render_target, returns) = f(self, render_target);

        unsafe {
            citro3d_sys::C3D_FrameEnd(0);
        }

        Ok((render_target.into(), returns))
    }
}

impl Instance {
    /// Change the render target for drawing the frame.
    /// This will activate the `new_target` (turning it into a [RenderTarget])
    /// and deactivate the `old_target` (turning it into a [ScreenTarget]).
    ///
    /// # Errors
    ///
    /// Fails if the `new_target` cannot be used for drawing.
    #[doc(alias = "C3D_FrameDrawOn")]
    pub fn swap_render_target<'screen, 'screen2, S: Screen + 'screen, S2: Screen + 'screen2>(
        &mut self,
        old_target: RenderTarget<'screen, S>,
        new_target: ScreenTarget<'screen2, S2>,
    ) -> std::result::Result<
        (ScreenTarget<'screen, S>, RenderTarget<'screen2, S2>),
        (RenderTarget<'screen, S>, ScreenTarget<'screen2, S2>, Error),
    > {
        match unsafe { self.set_render_target(&new_target) } {
            Ok(()) => Ok((old_target.into(), unsafe { new_target.into_inner() })),
            Err(e) => Err((old_target, new_target, e)),
        }
    }

    /// Sets the active render target.
    ///
    /// This function is unsafe because it doesn't deactivate the previous render target.
    /// You probably want to use [swap_render_target](Self::swap_render_target) instead.
    ///
    /// # Errors
    ///
    /// Fails if the `target` cannot be used for drawing.
    #[doc(alias = "C3D_FrameDrawOn")]
    pub unsafe fn set_render_target<S: Screen>(
        &mut self,
        target: &ScreenTarget<'_, S>,
    ) -> Result<()> {
        if unsafe { citro3d_sys::C3D_FrameDrawOn(target.get_inner_ref().as_raw()) } {
            Ok(())
        } else {
            Err(Error::InvalidRenderTarget)
        }
    }

    /// Get the buffer info being used, if it exists. Note that the resulting
    /// [`buffer::Info`] is copied from the one currently in use.
    #[doc(alias = "C3D_GetBufInfo")]
    pub fn buffer_info(&self) -> Option<buffer::Info> {
        let raw = unsafe { citro3d_sys::C3D_GetBufInfo() };
        buffer::Info::copy_from(raw)
    }

    /// Set the buffer info to use for any following draw calls.
    #[doc(alias = "C3D_SetBufInfo")]
    pub fn set_buffer_info(&mut self, buffer_info: &buffer::Info) {
        let raw: *const _ = &buffer_info.0;
        // SAFETY: C3D_SetBufInfo actually copies the pointee instead of mutating it.
        unsafe { citro3d_sys::C3D_SetBufInfo(raw.cast_mut()) };
    }

    /// Get the attribute info being used, if it exists. Note that the resulting
    /// [`attrib::Info`] is copied from the one currently in use.
    #[doc(alias = "C3D_GetAttrInfo")]
    pub fn attr_info(&self) -> Option<attrib::Info> {
        let raw = unsafe { citro3d_sys::C3D_GetAttrInfo() };
        attrib::Info::copy_from(raw)
    }

    /// Set the attribute info to use for any following draw calls.
    #[doc(alias = "C3D_SetAttrInfo")]
    pub fn set_attr_info(&mut self, attr_info: &attrib::Info) {
        let raw: *const _ = &attr_info.0;
        // SAFETY: C3D_SetAttrInfo actually copies the pointee instead of mutating it.
        unsafe { citro3d_sys::C3D_SetAttrInfo(raw.cast_mut()) };
    }

    /// Render primitives from the current vertex array buffer.
    #[doc(alias = "C3D_DrawArrays")]
    pub fn draw_arrays(&mut self, primitive: buffer::Primitive, vbo_data: buffer::Slice) {
        self.set_buffer_info(vbo_data.info());

        // TODO: should we also require the attrib info directly here?
        unsafe {
            citro3d_sys::C3D_DrawArrays(
                primitive as ctru_sys::GPU_Primitive_t,
                vbo_data.index(),
                vbo_data.len(),
            );
        }
    }
    /// Indexed drawing
    ///
    /// Draws the vertices in `buf` indexed by `indices`. `indices` must be linearly allocated
    ///
    /// # Safety
    // TODO: #41 might be able to solve this:
    /// If `indices` goes out of scope before the current frame ends it will cause a
    /// use-after-free (possibly by the GPU).
    ///
    /// # Panics
    ///
    /// If the given index buffer is too long to have its length converted to `i32`.
    #[doc(alias = "C3D_DrawElements")]
    pub unsafe fn draw_elements<I: Index>(
        &mut self,
        primitive: buffer::Primitive,
        vbo_data: buffer::Slice,
        indices: &Indices<'_, I>,
    ) {
        self.set_buffer_info(vbo_data.info());

        let indices = &indices.buffer;
        let elements = indices.as_ptr().cast();

        unsafe {
            citro3d_sys::C3D_DrawElements(
                primitive as ctru_sys::GPU_Primitive_t,
                indices.len().try_into().unwrap(),
                // flag bit for short or byte
                I::TYPE,
                elements,
            );
        }
    }

    /// Use the given [`shader::Program`] for subsequent draw calls.
    pub fn bind_program(&mut self, program: &shader::Program) {
        // SAFETY: AFAICT C3D_BindProgram just copies pointers from the given program,
        // instead of mutating the pointee in any way that would cause UB
        unsafe {
            citro3d_sys::C3D_BindProgram(program.as_raw().cast_mut());
        }
    }

    /// Binds a new [`LightEnv`], returning the previous one (if present).
    pub fn bind_light_env(
        &mut self,
        new_env: Option<Pin<Box<LightEnv>>>,
    ) -> Option<Pin<Box<LightEnv>>> {
        let old_env = self.light_env.take();
        self.light_env = new_env;

        unsafe {
            // setup the light env slot, since this is a pointer copy it will stick around even with we swap
            // out light_env later
            citro3d_sys::C3D_LightEnvBind(
                self.light_env
                    .as_mut()
                    .map_or(std::ptr::null_mut(), |env| env.as_mut().as_raw_mut()),
            );
        }

        old_env
    }

    pub fn light_env(&self) -> Option<Pin<&LightEnv>> {
        self.light_env.as_ref().map(|env| env.as_ref())
    }

    pub fn light_env_mut(&mut self) -> Option<Pin<&mut LightEnv>> {
        self.light_env.as_mut().map(|env| env.as_mut())
    }

    /// Bind a uniform to the given `index` in the vertex shader for the next draw call.
    ///
    /// # Example
    ///
    /// ```
    /// # let _runner = test_runner::GdbRunner::default();
    /// # use citro3d::uniform;
    /// # use citro3d::math::Matrix4;
    /// #
    /// # let mut instance = citro3d::Instance::new().unwrap();
    /// let idx = uniform::Index::from(0);
    /// let mtx = Matrix4::identity();
    /// instance.bind_vertex_uniform(idx, &mtx);
    /// ```
    pub fn bind_vertex_uniform(&mut self, index: uniform::Index, uniform: impl Into<Uniform>) {
        uniform.into().bind(self, shader::Type::Vertex, index);
    }

    /// Bind a uniform to the given `index` in the geometry shader for the next draw call.
    ///
    /// # Example
    ///
    /// ```
    /// # let _runner = test_runner::GdbRunner::default();
    /// # use citro3d::uniform;
    /// # use citro3d::math::Matrix4;
    /// #
    /// # let mut instance = citro3d::Instance::new().unwrap();
    /// let idx = uniform::Index::from(0);
    /// let mtx = Matrix4::identity();
    /// instance.bind_geometry_uniform(idx, &mtx);
    /// ```
    pub fn bind_geometry_uniform(&mut self, index: uniform::Index, uniform: impl Into<Uniform>) {
        uniform.into().bind(self, shader::Type::Geometry, index);
    }

    /// Retrieve the [`TexEnv`] for the given stage, initializing it first if necessary.
    ///
    /// # Example
    ///
    /// ```
    /// # use citro3d::texenv;
    /// # let _runner = test_runner::GdbRunner::default();
    /// # let mut instance = citro3d::Instance::new().unwrap();
    /// let stage0 = texenv::Stage::new(0).unwrap();
    /// let texenv0 = instance.texenv(stage0);
    /// ```
    #[doc(alias = "C3D_GetTexEnv")]
    #[doc(alias = "C3D_TexEnvInit")]
    pub fn texenv(&mut self, stage: texenv::Stage) -> &mut texenv::TexEnv {
        let texenv = &mut self.texenvs[stage.0];
        texenv.get_or_init(|| TexEnv::new(stage));
        // We have to do this weird unwrap to get a mutable reference,
        // since there is no `get_mut_or_init` or equivalent
        texenv.get_mut().unwrap()
    }
}

// This only exists to be an alias, which admittedly is kinda silly. The default
// impl should be equivalent though, since RenderQueue has a drop impl too.
impl Drop for Instance {
    #[doc(alias = "C3D_Fini")]
    fn drop(&mut self) {}
}

impl Drop for RenderQueue {
    fn drop(&mut self) {
        unsafe {
            citro3d_sys::C3D_Fini();
        }
    }
}

#[cfg(test)]
mod tests {
    use ctru::services::gfx::Gfx;

    use super::*;

    #[test]
    fn select_render_target() {
        let gfx = Gfx::new().unwrap();
        let top_screen = gfx.top_screen.borrow_mut();
        let bottom_screen = gfx.bottom_screen.borrow_mut();

        let mut instance = Instance::new().unwrap();
        let mut top_target = instance
            .create_screen_target(10, 10, top_screen, None)
            .unwrap();
        let mut bottom_target = instance
            .create_screen_target(10, 10, bottom_screen, None)
            .unwrap();

        (bottom_target, top_target) = instance
            .render_to_target(top_target, |instance, top_target| {
                let (top_screen_target, bottom_target) = instance
                    .swap_render_target(top_target, bottom_target)
                    .unwrap();
                (bottom_target, top_screen_target)
            })
            .unwrap();

        // Check that we don't get a double-free or use-after-free by dropping
        // the global instance before dropping the targets.
        drop(instance);
        drop(bottom_target);
        drop(top_target);
    }
}
