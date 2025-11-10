use std::ptr::null_mut;

use citro2d_sys::{C2D_Font, C2D_FontFree};
use ctru::error::ResultCode;
use ctru_sys::fontEnsureMapped;

/// Ensures the shared system font is mapped.
///
/// This function is only really useful when calling [`ctru_sys`] functions
/// directly in unsafe code.
///
/// Creating a [`Font`] (of any kind) does not ensure the shared system font
/// is mapped. (For more information, see [Safety](Font#safety))
///
/// Thus, before using any of its internals with [`ctru_sys`], it is
/// recommended to call this function first.
#[doc(alias = "fontEnsureMapped")]
pub fn ensure_shared_font_is_mapped() -> ctru::Result<()> {
    ResultCode(unsafe { fontEnsureMapped() })?;
    Ok(())
}

/// A citro2d font.
///
/// To get the "default font", use [`Font::get_shared`].
///
/// # Safety
/// This type wraps a pointer to a font that might be null.
/// Null represents the shared system font.
///
/// All functions in citro2d ensure the shared system font is mapped before
/// using it. Thus, always calling [`ensure_shared_font_is_mapped`] when
/// creating a `Font` would be redundant in safe code.
///
/// libctru doesn't always ensure this, however. It is advised to call
/// [`ensure_shared_font_is_mapped`] before using a [`ctru_sys::CFNT_s`]
/// pointer that might be null in unsafe code that uses [`ctru_sys`] directly.
///
/// You can get the inner [`C2D_Font`] pointer using [`Font::get_inner`].
#[derive(Debug, Clone, Default)]
pub struct Font(C2D_Font);

impl Font {
    /// Gets the region-native shared system font.
    /// Essentially, the "global default font".
    ///
    /// This is handled as a special case that internally uses a null pointer.
    ///
    /// Calling this is essentially free; the font is only ensured to be mapped
    /// when it actually gets used. You can have as many instances as you want.
    ///
    /// The font is always loaded, and shared system-wide.
    /// Dropping an instance of the shared system font won't free it.
    ///
    /// This function is equivalent to [`Font::default`].
    pub fn get_shared() -> Self {
        Font(null_mut())
    }

    /// Creates a `Font` from a [`C2D_Font`].
    ///
    /// # Safety
    /// The font data and pointer are not checked for validity.
    /// Ensuring their validity is up to the caller.
    ///
    /// [With the exception of null pointers](Self::get_shared), `Font`s
    /// created using this function will assume to have exclusive ownership
    /// over the underlying font. (You can't have more than one instance)
    ///
    /// Dropping this `Font` will free the pointee via [`C2D_FontFree`].
    pub unsafe fn from_raw(font_ptr: C2D_Font) -> Self {
        Self(font_ptr)
    }

    /// Gets a copy of the inner [`C2D_Font`] pointer.
    pub fn get_inner(&self) -> C2D_Font {
        self.0
    }
}

impl Drop for Font {
    /// Dropping a `Font` will free the underlying data using [`C2D_FontFree`].
    ///
    /// If the wrapped pointer is a null pointer, this does nothing.
    /// (You'd be attempting to free the shared system font)
    #[doc(alias = "C2D_FontFree")]
    fn drop(&mut self) {
        unsafe {
            C2D_FontFree(self.0);
        }
    }
}
