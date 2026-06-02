//! Cross-platform text clipboard helper for canvas-driven UI actions.

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardCopy {
    Copied,
    Requested,
    Failed,
}

#[cfg(target_arch = "wasm32")]
extern "C" {
    fn clipboard_write_text_extern(text: *const u8, len: usize) -> i32;
}

pub fn copy_text(text: &str) -> ClipboardCopy {
    macroquad::miniquad::window::clipboard_set(text);

    #[cfg(target_arch = "wasm32")]
    {
        match unsafe { clipboard_write_text_extern(text.as_ptr(), text.len()) } {
            2 => ClipboardCopy::Copied,
            1 => ClipboardCopy::Requested,
            _ => ClipboardCopy::Failed,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        ClipboardCopy::Copied
    }
}
