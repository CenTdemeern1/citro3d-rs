use std::{
    ffi::{CStr, c_double},
    mem::MaybeUninit,
    ptr::null_mut,
};

use citro2d_sys::{C2D_DrawText, C2D_Text, C2D_TextBuf, C2D_TextBufDelete, C2D_TextGetDimensions};
use citro3d::render::RenderTarget;

use crate::{
    Point, Size,
    drawable::{Drawable, DrawableResult},
    font::Font,
    render::Color,
};

/// Wrapper of [`C2D_TextBuf`], the glyph buffer for use with a  [`C2D_Text`] object.
///
/// Use [`C2D_TextParse`] to write to this buffer.
#[derive(Debug)]
pub struct TextGlyphBuffer {
    inner: C2D_TextBuf,
    /// The size of the buffer. citro2d doesn't expose any fields of [`C2D_TextBuf`],
    /// so we need to store this alongside it ourselves.
    buf_len: usize,
}

impl Drop for TextGlyphBuffer {
    /// Free the underlying data using [`C2D_TextBufDelete`] .
    #[doc(alias = "C2D_TextBufDelete")]
    fn drop(&mut self) {
        if !self.inner.is_null() {
            unsafe { C2D_TextBufDelete(self.inner) };
        }
    }
}

impl TextGlyphBuffer {
    /// Create a new citro2d text glyph buffer of a size.
    ///
    /// # Panics
    ///
    /// If C2D_TextBufNew() returns null.
    pub fn new(buf_len: usize) -> Self {
        let inner = unsafe { citro2d_sys::C2D_TextBufNew(buf_len) };
        assert!(
            !inner.is_null(),
            "creating C2D_TextBuf of size {buf_len} is null"
        );
        Self { inner, buf_len }
    }

    /// Resize the buffer to a new size.
    ///
    /// See [`Self::extend_to_text`] for resizing based on the glyph count of
    /// a text string.
    ///
    /// # Panics
    ///
    /// If C2D_TextBufResize() returns null.
    pub fn resize(&mut self, to: usize) {
        self.buf_len = 0;

        let new = unsafe { citro2d_sys::C2D_TextBufResize(self.inner, to) };
        assert!(!new.is_null(), "resizing C2D_TextBuf to {to} is null");

        self.inner = new;
        self.buf_len = to;
    }

    /// Extend the buffer if necessary to match the number of glyphs in `text`.
    ///
    /// This buffer is of glyphs and not graphemes, and whitespace is free.
    ///
    /// # Panics
    ///
    /// If C2D_TextBufResize() returns null.
    pub fn extend_to_text(&mut self, text: &CStr) {
        unsafe { citro2d_sys::C2D_TextBufClear(self.inner) };

        let mut to = text.count_bytes();
        let text_str = text.to_str().unwrap();
        if text_str.len() == to {
            to = text_str.chars().count() - text_str.matches([' ', '\n']).count();
        }

        if to > self.buf_len {
            self.resize(to.next_power_of_two());
        }
    }

    //// Gets a copy of the inner [`C2D_TextBuf`] pointer.
    ///
    /// # Safety
    ///
    /// If the buffer is dropped or resized, this pointer will dangle.
    pub unsafe fn get_inner(&self) -> C2D_TextBuf {
        self.inner
    }

    /// Get the internal buffer's length.
    pub fn len(&self) -> usize {
        self.buf_len
    }
    pub fn is_empty(&self) -> bool {
        self.buf_len == 0
    }
}

/// A citro2d text object.
#[derive(Debug)]
pub struct Text {
    inner: C2D_Text,
    buf: TextGlyphBuffer,
    pub point: Point,
    pub style: TextDrawStyle,
}
impl Text {
    pub fn new(point: Point, style: TextDrawStyle) -> Self {
        let buf = TextGlyphBuffer::new(0);

        // SAFETY: C2D_Text is OK to initialize with zeroed fields for all but `buf`.
        let mut inner = unsafe { MaybeUninit::<C2D_Text>::zeroed().assume_init() };
        inner.buf = buf.inner;

        Self {
            inner,
            buf,
            point,
            style,
        }
    }

    /// Parse text into the glyph buffer. Call this before rendering the text.
    #[doc(alias = "C2D_TextParse")]
    #[doc(alias = "C2D_TextFontParse")]
    pub fn parse(&mut self, text: &CStr, font: &Font) {
        let Self { inner, buf, .. } = self;

        buf.extend_to_text(text);

        if font.get_inner().is_null() {
            unsafe { citro2d_sys::C2D_TextParse(inner, buf.inner, text.as_ptr()) };
        } else {
            unsafe {
                citro2d_sys::C2D_TextFontParse(inner, font.get_inner(), buf.inner, text.as_ptr())
            };
        }
        unsafe { citro2d_sys::C2D_TextOptimize(inner) };
    }

    /// Roughly estimate the bounding box of the text object, which somewhat maps
    /// to the actual drawing dimensions. Works well only with the system font.
    ///
    /// This accounts for horizontal / vertical alignment and scalars. This does
    /// not account for word wrap or handle baseline alignment well.
    ///
    /// Returns the top left point and the size.
    #[doc(alias = "C2D_TextGetDimensions")]
    pub fn estimate_bounding(&self) -> (Point, Size) {
        let (mut w, mut h) = (0., 0.);
        let Point { mut x, mut y, z } = self.point;

        let (sx, sy) = self.style.scalars;
        unsafe { C2D_TextGetDimensions(&self.inner, sx, sy, &mut w, &mut h) };

        // Account for horizontal and vertical alignment.
        x = match self.style.horiz_align {
            HorizontalAlignment::Left | HorizontalAlignment::Justified => x,
            HorizontalAlignment::Center => x - w / 2.,
            HorizontalAlignment::Right => x - w,
        };
        y = match self.style.vert_align {
            VerticalAlignment::Top => y,
            VerticalAlignment::Center => y - h / 2.,
            VerticalAlignment::Baseline => y - h, // not accurate!
            VerticalAlignment::Bottom => y - h,
        };

        ((x, y, z).into(), (w, h).into())
    }
}

impl Drawable for Text {
    /// Draws a Text object to a render target given its style (`self.style`) and
    /// its underlying glyph buffer.
    ///
    /// See [`Self::parse`] for rendering a string with a font to the glyph buffer.
    ///
    /// Since [`C2D_DrawText`] has no result, this function always returns
    /// [`DrawableResult::Success`].
    #[doc(alias = "C2D_DrawText")]
    fn render(&self, _target: &mut RenderTarget<'_>) -> DrawableResult {
        let Point { x, mut y, z, .. } = self.point;
        // Vertical center and bottom alignment is not handled by citro2d.
        if matches!(
            self.style.vert_align,
            VerticalAlignment::Center | VerticalAlignment::Bottom
        ) {
            (Point { y, .. }, _) = self.estimate_bounding();
        }

        #[cfg_attr(any(), rustfmt::skip)]
        if let Some(wrap) = self.style.word_wrap {
            unsafe {
                C2D_DrawText(
                    &self.inner, self.style.flags(), x, y, z, self.style.scalars.0, self.style.scalars.1, self.style.color,
                    wrap as c_double,
                )
            };
        } else {
            unsafe {
                C2D_DrawText(
                    &self.inner, self.style.flags(), x, y, z, self.style.scalars.0, self.style.scalars.1, self.style.color,
                )
            };
        }

        DrawableResult::Success
    }
}

/// Horizontal alignment for a text's style.
///
/// The value associated with each variant corresponds to its flag.
#[repr(u8)]
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum HorizontalAlignment {
    /// The point's X coordinate is the left-most edge.
    #[default]
    Left = citro2d_sys::C2D_AlignLeft,
    /// The point's X coordinate is the right-most edge.
    Right = citro2d_sys::C2D_AlignRight,
    /// The point's X coordinate is the center of the text.
    Center = citro2d_sys::C2D_AlignCenter,
    /// The point's X coordinate is the left-most edge.
    /// To be used in conjunction with a word wrap. Otherwise, this is [`Self::Left`].
    Justified = citro2d_sys::C2D_AlignJustified,
}
/// Vertical alignment for a text's style.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerticalAlignment {
    /// The point's Y coordinate is the baseline of the font.
    Baseline,
    /// The point's Y coordinate is the top-most edge.
    #[default]
    Top,
    /// The point's Y coordinate is the center.
    Center,
    /// The point's Y coordinate is the bottom-most edge. This differs from the
    /// baseline of the font. For example, rendering a ',' will go under the
    /// baseline of the font but is at or above the bottom-most edge of the text.
    Bottom,
}

#[derive(Debug, Clone, Copy)]
pub struct TextDrawStyle {
    pub horiz_align: HorizontalAlignment,
    pub vert_align: VerticalAlignment,
    /// X, Y scalars from the original font bitmap size.
    pub scalars: (f32, f32),
    /// Enables word wrap at spaces with a given maximum width for wrapping.
    pub word_wrap: Option<f32>,
    pub color: Color,
}
impl Default for TextDrawStyle {
    fn default() -> Self {
        Self {
            horiz_align: HorizontalAlignment::default(),
            vert_align: VerticalAlignment::default(),
            scalars: (1., 1.),
            word_wrap: None,
            color: Color::new(0, 0, 0),
        }
    }
}
impl TextDrawStyle {
    pub const fn with_scalar(self, scalar: f32) -> Self {
        Self {
            scalars: (scalar, scalar),
            ..self
        }
    }
    pub const fn with_scalars(self, scalar_x: f32, scalar_y: f32) -> Self {
        Self {
            scalars: (scalar_x, scalar_y),
            ..self
        }
    }
    pub fn with_color(self, color: impl Into<Color>) -> Self {
        Self {
            color: color.into(),
            ..self
        }
    }
    pub const fn with_horizontal_alignment(self, horiz_align: HorizontalAlignment) -> Self {
        Self {
            horiz_align,
            ..self
        }
    }
    pub const fn with_vertical_alignment(self, vert_align: VerticalAlignment) -> Self {
        Self { vert_align, ..self }
    }
    pub const fn with_alignments(
        self,
        horiz_align: HorizontalAlignment,
        vert_align: VerticalAlignment,
    ) -> Self {
        Self {
            horiz_align,
            vert_align,
            ..self
        }
    }
    pub const fn with_word_wrap(self, wrap: f32) -> Self {
        Self {
            word_wrap: Some(wrap),
            ..self
        }
    }
    pub const fn without_word_wrap(self) -> Self {
        Self {
            word_wrap: None,
            ..self
        }
    }

    /// Flags for `C2D_DrawText`.
    pub fn flags(&self) -> u32 {
        // Bit 0: at baseline
        // Bit 1: with color
        // Bits 2-3: horizontal alignment mask
        // Bit 4: word wrap
        (self.vert_align == VerticalAlignment::Baseline) as u32
            | ((self.color.inner != 0x000000ff) as u32) << 1
            | (self.horiz_align as u32) // already bitshifted
            | ((self.word_wrap.is_some() as u32) << 4)
    }
}
