use std::{
    ffi::{CString, c_double},
    io, mem,
    ptr::NonNull,
};

use citro2d_sys::C2D_FontGetInfo;
use citro2d_sys::{C2D_DrawText, C2D_Text, C2D_TextGetDimensions};
use citro2d_sys::{
    C2D_TextBuf, C2D_TextBuf_s, C2D_TextBufClear, C2D_TextBufDelete, C2D_TextBufGetNumGlyphs,
    C2D_TextBufNew, C2D_TextBufResize,
};
use citro3d::render::RenderTarget;

use crate::{
    Point, Size,
    drawable::{Drawable, DrawableResult},
    font::Font,
    render::Color,
};

/// A citro2d text glyph buffer, for use with [`Text`]. Stores the glyphs for a
/// string and their positioning.
#[derive(Debug)]
pub struct TextGlyphBuffer {
    inner: NonNull<C2D_TextBuf_s>,
    /// The capacity of the buffer in glyphs.
    ///
    /// citro2d doesn't expose any fields of [`C2D_TextBuf`], so we need to store
    /// this ourselves. Equal to `C2D_TextBuf->glyphBufSize`.
    capacity: usize,
}

impl Drop for TextGlyphBuffer {
    /// Dropping a `TextGlyphBuffer` will free the underlying data using
    /// [`C2D_TextBufDelete`] .
    #[doc(alias = "C2D_TextBufDelete")]
    fn drop(&mut self) {
        unsafe { C2D_TextBufDelete(self.inner.as_ptr()) };
    }
}

/// A citro2d text buffer.
///
/// A text object uses this buffer to store character glyphs and positioning.
/// Use [`Text::parse`] to "render" to this buffer.
impl TextGlyphBuffer {
    /// Creates a new citro2d text buffer.
    #[doc(alias = "C2D_TextBufNew")]
    pub fn new(max_glyphs: usize) -> io::Result<Self> {
        let inner = NonNull::new(unsafe { C2D_TextBufNew(max_glyphs) }).ok_or_else(|| {
            io::Error::new(
                io::ErrorKind::OutOfMemory,
                format!("failed to make a C2D text buffer of {max_glyphs} glyphs"),
            )
        })?;

        Ok(Self {
            inner,
            capacity: max_glyphs,
        })
    }

    /// Clears the buffer.
    pub fn clear(&mut self) {
        unsafe { C2D_TextBufClear(self.inner.as_ptr()) };
    }

    /// Gets the number of glyphs currently stored in the buffer. See
    /// [`Self::capacity`] for the maximum number of glyphs it can store.
    #[doc(alias = "C2D_TextBufGetNumGlyphs")]
    pub fn get_num_glyphs(&self) -> usize {
        // This doesn't mutate the buffer.
        unsafe { C2D_TextBufGetNumGlyphs(self.inner.as_ptr()) }
    }

    /// Gets the internal buffer's capacity, i.e. the maximum number of glyphs
    /// it can store.
    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Resizes the buffer to a new capacity, copying over the glyphs from the
    /// current buffer.
    ///
    /// If the new capacity is less than the buffer's length, the buffer will
    /// truncate.
    #[doc(alias = "C2D_TextBufResize")]
    pub fn resize(&mut self, max_glyphs: usize) -> io::Result<()> {
        let new = NonNull::new(unsafe { C2D_TextBufResize(self.inner.as_ptr(), max_glyphs) })
            .ok_or(io::Error::new(
                io::Error::last_os_error().kind(),
                format!("failed to resize C2D text buffer to {max_glyphs} glyphs"),
            ))?;

        self.inner = new;
        self.capacity = max_glyphs;

        Ok(())
    }

    /// Resizes the buffer to only fit the number of glyphs it contains.
    pub fn shrink_to_fit(&mut self) -> io::Result<()> {
        self.resize(self.get_num_glyphs())?;

        Ok(())
    }

    /// Extends the buffer to fit at least the number of glyphs in `text`
    /// (whitespace is free).
    ///
    /// If the number of glyphs in `text` is more than the buffer's capacity,
    /// the buffer is extended to the next power of two that fits.
    ///
    /// Otherwise, this function does nothing.
    pub fn extend_to_text(&mut self, text: &str) -> io::Result<()> {
        let to = text.chars().count() - text.matches([' ', '\n']).count();
        if to > self.capacity {
            self.resize(to.next_power_of_two())?;
        }

        Ok(())
    }

    /// Gets a copy of the inner [`C2D_TextBuf`] pointer.
    pub fn get_inner(&self) -> C2D_TextBuf {
        self.inner.as_ptr()
    }
}

/// A citro2d text object.
#[derive(Debug)]
pub struct Text {
    inner: C2D_Text,
    buf: TextGlyphBuffer,
    pub position: Point,
    pub style: TextDrawStyle,
}

impl Text {
    pub fn new(point: Point, style: TextDrawStyle) -> io::Result<Self> {
        let buf = TextGlyphBuffer::new(0)?;

        // SAFETY: C2D_Text is OK to initialize with zeroed fields for all but `buf`.
        let mut inner = unsafe { mem::zeroed::<C2D_Text>() };
        inner.buf = buf.get_inner();

        Ok(Self {
            inner,
            buf,
            position: point,
            style,
        })
    }

    /// Parses text into the glyph buffer.
    ///
    /// Call this before rendering the text.
    #[doc(alias = "C2D_TextParse")]
    #[doc(alias = "C2D_TextFontParse")]
    pub fn parse(&mut self, text: &str, font: &Font) -> io::Result<()> {
        let Self { inner, buf, .. } = self;

        buf.extend_to_text(text)?;
        let c_str = CString::new(text)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "NUL in string"))?;

        unsafe {
            citro2d_sys::C2D_TextFontParse(inner, font.get_inner(), buf.get_inner(), c_str.as_ptr())
        };
        unsafe { citro2d_sys::C2D_TextOptimize(inner) };

        Ok(())
    }

    /// Gets roughly the size of the text. Works well only with the system font.
    #[doc(alias = "C2D_TextGetDimensions")]
    pub fn get_dimensions(&self) -> Size {
        let (sx, sy) = self.style.scale;

        let (mut w, mut h) = (0., 0.);
        unsafe { C2D_TextGetDimensions(&self.inner, sx, sy, &mut w, &mut h) };

        (w, h).into()
    }

    /// Gets roughly the bounding box of the text object. Works well only with
    /// the system font.
    ///
    /// This accounts for horizontal / vertical alignment and scale. This does
    /// not account for word wrap.
    ///
    /// Returns the top left point and the size.
    pub fn estimate_bounding_box(&self, font: &Font) -> (Point, Size) {
        let Size {
            width: w,
            height: h,
        } = self.get_dimensions();

        let Point { mut x, mut y, z } = self.position;

        // Account for horizontal and vertical alignment.
        x = match self.style.horiz_align {
            HorizontalAlignment::Left | HorizontalAlignment::Justified => x,
            HorizontalAlignment::Center => x - w / 2.,
            HorizontalAlignment::Right => x - w,
        };
        y = match self.style.vert_align {
            VerticalAlignment::Top => y,
            VerticalAlignment::Center => y - h / 2.,
            VerticalAlignment::Baseline => unsafe {
                let finf = *C2D_FontGetInfo(font.get_inner());
                let tglp = *finf.tglp;
                // Baseline pos is measured from the top of the text.
                let baseline_pos = tglp.baselinePos;
                y - (baseline_pos as f32)
            },
            VerticalAlignment::Bottom => y - h,
        };

        ((x, y, z).into(), (w, h).into())
    }

    /// Consumes the text into its underlying buffer.
    pub fn into_buffer(self) -> TextGlyphBuffer {
        self.buf
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
        let Point { x, mut y, z, .. } = self.position;

        // Vertical alignments center and bottom are not handled by citro2d.
        let Size { height, .. } = self.get_dimensions();
        if self.style.vert_align == VerticalAlignment::Center {
            y -= height / 2.;
        } else if self.style.vert_align == VerticalAlignment::Bottom {
            y -= height;
        }

        if let Some(wrap) = self.style.word_wrap {
            unsafe {
                C2D_DrawText(
                    &self.inner,
                    self.style.flags(),
                    x,
                    y,
                    z,
                    self.style.scale.0,
                    self.style.scale.1,
                    self.style.color,
                    wrap as c_double,
                )
            };
        } else {
            unsafe {
                C2D_DrawText(
                    &self.inner,
                    self.style.flags(),
                    x,
                    y,
                    z,
                    self.style.scale.0,
                    self.style.scale.1,
                    self.style.color,
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
    pub scale: (f32, f32),
    /// Enables word wrap at spaces with a given maximum width for wrapping.
    pub word_wrap: Option<f32>,
    pub color: Color,
}

impl Default for TextDrawStyle {
    fn default() -> Self {
        Self {
            horiz_align: HorizontalAlignment::default(),
            vert_align: VerticalAlignment::default(),
            scale: (1., 1.),
            word_wrap: None,
            color: Color::new(0, 0, 0),
        }
    }
}

impl TextDrawStyle {
    pub const fn with_x_y_scales(self, scalar_x: f32, scalar_y: f32) -> Self {
        Self {
            scale: (scalar_x, scalar_y),
            ..self
        }
    }

    pub const fn with_scale(self, scalar: f32) -> Self {
        Self {
            scale: (scalar, scalar),
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

    pub const fn with_word_wrap(self, wrap: Option<f32>) -> Self {
        Self {
            word_wrap: wrap,
            ..self
        }
    }

    /// Returns the flags used for [`C2D_DrawText`].
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
