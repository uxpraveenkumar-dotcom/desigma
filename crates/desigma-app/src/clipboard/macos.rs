//! Read SVG bytes from the native pasteboard before falling back to text or images.

use objc2::{class, msg_send_id, rc::Retained, runtime::NSObject};
use objc2_foundation::{NSData, NSString};

pub(super) fn svg() -> Option<String> {
    // Paste runs on the UI thread, where AppKit's general pasteboard is available.
    let pasteboard: Option<Retained<NSObject>> = unsafe { msg_send_id![class!(NSPasteboard), generalPasteboard] };
    let pasteboard = pasteboard?;
    for format in ["public.svg-image", "image/svg+xml"] {
        let format = NSString::from_str(format);
        let data: Option<Retained<NSData>> = unsafe { msg_send_id![&*pasteboard, dataForType: &*format] };
        if let Some(data) = data {
            let Ok(svg) = std::str::from_utf8(data.bytes()) else { continue };
            let svg = svg.trim_end_matches('\0');
            if desigma_svg::looks_like_svg(svg) {
                return Some(svg.to_owned());
            }
        }
    }
    None
}
