use std::{
    ffi::CString,
    io,
    os::{fd::AsRawFd, unix::ffi::OsStrExt},
    path::Path,
    ptr::null_mut,
};

use citro2d_sys::{C2D_Font, C2D_FontFree, C2D_FontLoad, C2D_FontLoadFromFD, C2D_FontLoadFromMem};
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
        Font(font_ptr)
    }

    /// Load a font from a file path.
    ///
    /// This expects a `.bcfnt` CTR bitmap font, not your typical TTF file; for
    /// more information about this format
    /// [see 3dbrew](https://www.3dbrew.org/wiki/BCFNT).
    ///
    /// # Safety
    /// The font data is not checked for validity. Ensuring the validity of the
    /// font data is up to the caller.
    #[doc(alias = "C2D_FontLoad")]
    pub unsafe fn from_file_path_unchecked(path: impl AsRef<Path>) -> io::Result<Self> {
        // Turn the path into a CString
        let c_path = CString::new(path.as_ref().as_os_str().as_bytes())
            .map_err(|_| io::ErrorKind::InvalidFilename)?;

        // Actually load the font
        Self::pointer_or_last_error(unsafe { C2D_FontLoad(c_path.as_ptr()) })
    }

    /// Load a font from a file descriptor.
    ///
    /// # Safety
    /// The font data is not checked for validity. Ensuring the validity of the
    /// font data is up to the caller.
    ///
    /// Ensuring the safety of actually using the file descriptor is also up to
    /// the caller.
    #[doc(alias = "C2D_FontLoadFromFD")]
    pub unsafe fn from_file_descriptor_unchecked(fd: impl AsRawFd) -> io::Result<Self> {
        Self::pointer_or_last_error(unsafe { C2D_FontLoadFromFD(fd.as_raw_fd()) })
    }

    /// Load a font from a slice.
    ///
    /// # Safety
    /// The font data is not checked for validity. Ensuring the validity of the
    /// font data is up to the caller.
    #[doc(alias = "C2D_FontLoadFromMem")]
    pub unsafe fn from_slice_unchecked(slice: &[u8]) -> io::Result<Self> {
        Self::pointer_or_last_error(unsafe {
            C2D_FontLoadFromMem(slice.as_ptr().cast(), slice.len())
        })
    }

    fn pointer_or_last_error(font_pointer: C2D_Font) -> io::Result<Self> {
        if font_pointer.is_null() {
            Err(io::Error::last_os_error())
        } else {
            Ok(Font(font_pointer))
        }
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
