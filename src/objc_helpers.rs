use std::ffi::{c_char, c_void, CStr};

use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

pub const UTF8_ENCODING: usize = 4;

/// `&str` から NSString を生成して返す。
///
/// # Safety
/// 返されたポインタは `alloc/init` で手動管理されるため、
/// 呼び出し側が `msg_send![ptr, release]` で解放する責任を持つ。
pub unsafe fn nsstring_from_str(value: &str) -> *mut AnyObject {
    let ns_string: *mut AnyObject = msg_send![class!(NSString), alloc];
    msg_send![
        ns_string,
        initWithBytes: value.as_ptr() as *const c_void
        length: value.len()
        encoding: UTF8_ENCODING
    ]
}

pub unsafe fn nsstring_to_string(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let utf8_ptr: *const c_char = msg_send![value, UTF8String];
    if utf8_ptr.is_null() {
        return Some(String::new());
    }

    Some(CStr::from_ptr(utf8_ptr).to_string_lossy().into_owned())
}
