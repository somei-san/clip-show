use std::ffi::{c_char, c_void, CStr};
use std::ptr;
use std::sync::{Mutex, Once};

use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{class, msg_send, sel};
use objc2_foundation::{NSPoint, NSRect, NSSize};

const UTF8_ENCODING: usize = 4;
const POLL_INTERVAL_SECS: f64 = 0.3;
const HUD_DURATION_SECS: f64 = 1.0;
const HUD_WIDTH: f64 = 700.0;
const HUD_HEIGHT: f64 = 160.0;
const BORDERLESS_MASK: usize = 0;
const BACKING_BUFFERED: isize = 2;
const FLOATING_WINDOW_LEVEL: isize = 3;

struct AppState {
    last_change_count: isize,
    pasteboard: *mut AnyObject,
    window: *mut AnyObject,
    label: *mut AnyObject,
    hide_timer: *mut AnyObject,
}

// All UI interactions happen on the AppKit main thread.
unsafe impl Send for AppState {}

static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

fn main() {
    unsafe {
        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        let _: bool = msg_send![app, setActivationPolicy: 1isize];

        let delegate_class = get_delegate_class();
        let delegate: *mut AnyObject = msg_send![delegate_class, new];
        let () = msg_send![app, setDelegate: delegate];
        let () = msg_send![app, run];
    }
}

fn get_delegate_class() -> &'static AnyClass {
    static ONCE: Once = Once::new();
    static mut CLASS: *const AnyClass = ptr::null();

    ONCE.call_once(|| unsafe {
        let mut builder = ClassBuilder::new("ClipboardHudAppDelegate", class!(NSObject))
            .expect("delegate class creation failed");

        builder.add_method(
            sel!(applicationDidFinishLaunching:),
            application_did_finish_launching as extern "C" fn(_, _, _),
        );
        builder.add_method(
            sel!(pollPasteboard:),
            poll_pasteboard as extern "C" fn(_, _, _),
        );
        builder.add_method(
            sel!(hideHud:),
            hide_hud as extern "C" fn(_, _, _),
        );

        let class = builder.register();
        CLASS = class as *const AnyClass;
    });

    unsafe { &*CLASS }
}

extern "C" fn application_did_finish_launching(this: &AnyObject, _: Sel, _: *mut AnyObject) {
    unsafe {
        let pasteboard: *mut AnyObject = msg_send![class!(NSPasteboard), generalPasteboard];
        let last_change_count: isize = msg_send![pasteboard, changeCount];

        let (window, label) = create_hud_window();

        *APP_STATE.lock().expect("APP_STATE lock poisoned") = Some(AppState {
            last_change_count,
            pasteboard,
            window,
            label,
            hide_timer: ptr::null_mut(),
        });

        let _: *mut AnyObject = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: POLL_INTERVAL_SECS
            target: this
            selector: sel!(pollPasteboard:)
            userInfo: ptr::null_mut::<AnyObject>()
            repeats: true
        ];
    }
}

extern "C" fn poll_pasteboard(this: &AnyObject, _: Sel, _: *mut AnyObject) {
    unsafe {
        let mut guard = APP_STATE.lock().expect("APP_STATE lock poisoned");
        let Some(state) = guard.as_mut() else {
            return;
        };

        let change_count: isize = msg_send![state.pasteboard, changeCount];
        if change_count == state.last_change_count {
            return;
        }
        state.last_change_count = change_count;

        let text_type = nsstring_from_str("public.utf8-plain-text");
        let raw_text: *mut AnyObject = msg_send![state.pasteboard, stringForType: text_type];
        let () = msg_send![text_type, release];

        let Some(text) = nsstring_to_string(raw_text) else {
            return;
        };

        let truncated = truncate_text(&text, 100, 5);
        let display = format!("📋 {}", truncated);

        let message = nsstring_from_str(&display);
        let () = msg_send![state.label, setStringValue: message];
        let () = msg_send![message, release];

        center_window(state.window, HUD_WIDTH, HUD_HEIGHT);
        let () = msg_send![state.window, orderFrontRegardless];

        if !state.hide_timer.is_null() {
            let () = msg_send![state.hide_timer, invalidate];
        }

        let hide_timer: *mut AnyObject = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: HUD_DURATION_SECS
            target: this
            selector: sel!(hideHud:)
            userInfo: ptr::null_mut::<AnyObject>()
            repeats: false
        ];
        state.hide_timer = hide_timer;
    }
}

extern "C" fn hide_hud(_: &AnyObject, _: Sel, _: *mut AnyObject) {
    unsafe {
        let mut guard = APP_STATE.lock().expect("APP_STATE lock poisoned");
        let Some(state) = guard.as_mut() else {
            return;
        };

        let () = msg_send![state.window, orderOut: ptr::null_mut::<AnyObject>()];

        if !state.hide_timer.is_null() {
            let () = msg_send![state.hide_timer, invalidate];
            state.hide_timer = ptr::null_mut();
        }
    }
}

unsafe fn create_hud_window() -> (*mut AnyObject, *mut AnyObject) {
    let mut rect = NSRect {
        origin: NSPoint { x: 0.0, y: 0.0 },
        size: NSSize {
            width: HUD_WIDTH,
            height: HUD_HEIGHT,
        },
    };

    if let Some((x, y)) = centered_origin(HUD_WIDTH, HUD_HEIGHT) {
        rect.origin = NSPoint { x, y };
    }

    let window: *mut AnyObject = msg_send![class!(NSWindow), alloc];
    let window: *mut AnyObject = msg_send![
        window,
        initWithContentRect: rect
        styleMask: BORDERLESS_MASK
        backing: BACKING_BUFFERED
        defer: false
    ];

    let () = msg_send![window, setOpaque: false];
    let () = msg_send![window, setHasShadow: true];
    let () = msg_send![window, setIgnoresMouseEvents: true];
    let () = msg_send![window, setLevel: FLOATING_WINDOW_LEVEL];

    let clear: *mut AnyObject = msg_send![class!(NSColor), clearColor];
    let () = msg_send![window, setBackgroundColor: clear];

    let content_view: *mut AnyObject = msg_send![window, contentView];
    let () = msg_send![content_view, setWantsLayer: true];
    let layer: *mut AnyObject = msg_send![content_view, layer];
    let () = msg_send![layer, setCornerRadius: 6.0f64];
    let () = msg_send![layer, setMasksToBounds: true];

    let bg: *mut AnyObject = msg_send![class!(NSColor), colorWithCalibratedWhite: 0.0f64 alpha: 0.8f64];
    let cg_color: *mut c_void = msg_send![bg, CGColor];
    let () = msg_send![layer, setBackgroundColor: cg_color];

    let label_rect = NSRect {
        origin: NSPoint { x: 20.0, y: 20.0 },
        size: NSSize {
            width: HUD_WIDTH - 40.0,
            height: HUD_HEIGHT - 40.0,
        },
    };

    let label: *mut AnyObject = msg_send![class!(NSTextField), alloc];
    let label: *mut AnyObject = msg_send![label, initWithFrame: label_rect];

    let () = msg_send![label, setBezeled: false];
    let () = msg_send![label, setBordered: false];
    let () = msg_send![label, setEditable: false];
    let () = msg_send![label, setSelectable: false];
    let () = msg_send![label, setDrawsBackground: false];
    let () = msg_send![label, setLineBreakMode: 0isize];
    let () = msg_send![label, setUsesSingleLineMode: false];
    let () = msg_send![label, setMaximumNumberOfLines: 5isize];
    let () = msg_send![label, setAlignment: 1isize];

    let white: *mut AnyObject = msg_send![class!(NSColor), whiteColor];
    let () = msg_send![label, setTextColor: white];

    let menlo_name = nsstring_from_str("Menlo");
    let font: *mut AnyObject = msg_send![class!(NSFont), fontWithName: menlo_name size: 18.0f64];
    let () = msg_send![menlo_name, release];
    if !font.is_null() {
        let () = msg_send![label, setFont: font];
    }

    let cell: *mut AnyObject = msg_send![label, cell];
    if !cell.is_null() {
        let () = msg_send![cell, setWraps: true];
        let () = msg_send![cell, setScrollable: false];
        let () = msg_send![cell, setLineBreakMode: 0isize];
    }

    let default_text = nsstring_from_str("📋");
    let () = msg_send![label, setStringValue: default_text];
    let () = msg_send![default_text, release];

    let () = msg_send![content_view, addSubview: label];
    let () = msg_send![window, orderOut: ptr::null_mut::<AnyObject>()];

    (window, label)
}

unsafe fn centered_origin(width: f64, height: f64) -> Option<(f64, f64)> {
    let screen: *mut AnyObject = msg_send![class!(NSScreen), mainScreen];
    if screen.is_null() {
        return None;
    }

    let frame: NSRect = msg_send![screen, frame];
    let x = frame.origin.x + (frame.size.width - width) / 2.0;
    let y = frame.origin.y + (frame.size.height - height) / 2.0;
    Some((x, y))
}

unsafe fn center_window(window: *mut AnyObject, width: f64, height: f64) {
    let Some((x, y)) = centered_origin(width, height) else {
        return;
    };

    let rect = NSRect {
        origin: NSPoint { x, y },
        size: NSSize { width, height },
    };
    let () = msg_send![window, setFrame: rect display: true];
}

unsafe fn nsstring_from_str(value: &str) -> *mut AnyObject {
    let ns_string: *mut AnyObject = msg_send![class!(NSString), alloc];
    msg_send![
        ns_string,
        initWithBytes: value.as_ptr() as *const c_void
        length: value.len()
        encoding: UTF8_ENCODING
    ]
}

unsafe fn nsstring_to_string(value: *mut AnyObject) -> Option<String> {
    if value.is_null() {
        return None;
    }

    let utf8_ptr: *const c_char = msg_send![value, UTF8String];
    if utf8_ptr.is_null() {
        return Some(String::new());
    }

    Some(CStr::from_ptr(utf8_ptr).to_string_lossy().into_owned())
}

fn truncate_text(text: &str, max_width: usize, max_lines: usize) -> String {
    let mut lines: Vec<String> = text
        .split('\n')
        .map(|line| truncate_line(line, max_width))
        .collect();

    if lines.len() > max_lines {
        lines.truncate(max_lines);
        if let Some(last) = lines.last_mut() {
            *last = append_ellipsis(last, max_width);
        }
    }

    lines.join("\n")
}

fn truncate_line(line: &str, max_width: usize) -> String {
    let count = line.chars().count();
    if count <= max_width {
        return line.to_string();
    }

    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }

    let kept: String = line.chars().take(max_width - 3).collect();
    format!("{kept}...")
}

fn append_ellipsis(line: &str, max_width: usize) -> String {
    if max_width == 0 {
        return String::new();
    }

    if max_width <= 3 {
        return "...".chars().take(max_width).collect();
    }

    let current_len = line.chars().count();
    if current_len + 3 <= max_width {
        return format!("{line}...");
    }

    let kept: String = line.chars().take(max_width - 3).collect();
    format!("{kept}...")
}

#[cfg(test)]
mod tests {
    use super::truncate_text;

    #[test]
    fn truncates_single_long_line() {
        let input = "abcdefghijklmnopqrstuvwxyz";
        assert_eq!(truncate_text(input, 10, 5), "abcdefg...");
    }

    #[test]
    fn truncates_lines_count_and_adds_ellipsis_to_last_line() {
        let input = "line1\nline2\nline3\nline4\nline5\nline6";
        assert_eq!(truncate_text(input, 100, 5), "line1\nline2\nline3\nline4\nline5...");
    }

    #[test]
    fn handles_utf8_by_char_count() {
        let input = "あいうえおかきくけこ";
        assert_eq!(truncate_text(input, 6, 5), "あいう...");
    }
}
