//! The functions `web/rockuml.js` calls. JavaScript writes the source to memory it allocates with
//! `rockuml_alloc`, calls `rockuml_render`, then reads the rendering through the `rockuml_output_*` functions
//! before the next call replaces it.
#![allow(unsafe_code)]

use std::cell::RefCell;
use std::sync::{Arc, LazyLock};

use rockuml::fonts::FontRegistry;

use crate::wasm_host::WasmHost;
use crate::{Format, Rendering, Status, render};

/// Parsed once, as every rendering measures text with them.
static FONTS: LazyLock<Arc<FontRegistry>> = LazyLock::new(Arc::default);

thread_local! {
    static OUTPUT: RefCell<Rendering> = const {
        RefCell::new(Rendering {
            status: Status::NoImage,
            data: Vec::new(),
            page_count: 0,
        })
    };
}

/// `length` bytes for JavaScript to fill; it hands them back with `rockuml_free`.
#[unsafe(no_mangle)]
pub extern "C" fn rockuml_alloc(length: usize) -> *mut u8 {
    Box::into_raw(vec![0_u8; length].into_boxed_slice()).cast()
}

/// # Safety
///
/// `pointer` and `length` are those of a `rockuml_alloc` not yet freed.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rockuml_free(pointer: *mut u8, length: usize) {
    // SAFETY: the caller hands back a boxed slice of this length that `rockuml_alloc` leaked.
    drop(unsafe { Box::from_raw(std::ptr::slice_from_raw_parts_mut(pointer, length)) });
}

/// Renders one page of the UTF-8 `source` in the format with this code (see `Format`) and returns the
/// status code (see `Status`).
///
/// # Safety
///
/// `source` points to `length` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn rockuml_render(
    source: *const u8,
    length: usize,
    format: u32,
    page: usize,
) -> u32 {
    // SAFETY: the caller guarantees the bytes are readable.
    let bytes = unsafe { std::slice::from_raw_parts(source, length) };
    let rendering = match Format::from_code(format) {
        Some(format) => render(
            &String::from_utf8_lossy(bytes),
            format,
            page,
            &WasmHost,
            &FONTS,
        ),
        None => Rendering {
            status: Status::NoImage,
            data: format!("there is no format {format}").into_bytes(),
            page_count: 0,
        },
    };
    let status = rendering.status as u32;
    OUTPUT.set(rendering);
    status
}

/// Where the last rendering's image, text or message starts.
#[unsafe(no_mangle)]
pub extern "C" fn rockuml_output_pointer() -> *const u8 {
    OUTPUT.with_borrow(|output| output.data.as_ptr())
}

#[unsafe(no_mangle)]
pub extern "C" fn rockuml_output_length() -> usize {
    OUTPUT.with_borrow(|output| output.data.len())
}

/// How many pages the last rendered source has.
#[unsafe(no_mangle)]
pub extern "C" fn rockuml_page_count() -> usize {
    OUTPUT.with_borrow(|output| output.page_count)
}
