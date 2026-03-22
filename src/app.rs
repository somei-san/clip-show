use std::ptr;
use std::sync::{Mutex, Once};

use objc2::declare::ClassBuilder;
use objc2::runtime::{AnyClass, AnyObject, Sel};
use objc2::{class, msg_send, sel};

use crate::config::{display_settings, DisplaySettings};
use crate::hud::{create_hud_window, layout_hud};
use crate::objc_helpers::nsstring_from_str;
use crate::text::truncate_text;

pub const FADE_TICK_INTERVAL_SECS: f64 = 1.0 / 60.0;

pub struct AppState {
    pub last_change_count: isize,
    pub pasteboard: *mut AnyObject,
    pub window: *mut AnyObject,
    pub icon_label: *mut AnyObject,
    pub label: *mut AnyObject,
    pub hide_timer: *mut AnyObject,
    pub fade_timer: *mut AnyObject,
    pub fade_ticks_elapsed: u32,
    pub fade_total_ticks: u32,
    pub settings: DisplaySettings,
}

// All UI interactions happen on the AppKit main thread.
unsafe impl Send for AppState {}

pub static APP_STATE: Mutex<Option<AppState>> = Mutex::new(None);

pub fn get_delegate_class() -> &'static AnyClass {
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
        builder.add_method(sel!(hideHud:), hide_hud as extern "C" fn(_, _, _));
        builder.add_method(sel!(fadeTick:), fade_tick as extern "C" fn(_, _, _));

        let class = builder.register();
        CLASS = class as *const AnyClass;
    });

    unsafe { &*CLASS }
}

extern "C" fn application_did_finish_launching(this: &AnyObject, _: Sel, _: *mut AnyObject) {
    unsafe {
        let settings = display_settings();
        let pasteboard: *mut AnyObject = msg_send![class!(NSPasteboard), generalPasteboard];
        let last_change_count: isize = msg_send![pasteboard, changeCount];

        let (window, icon_label, label) = create_hud_window(settings);

        *APP_STATE.lock().expect("APP_STATE lock poisoned") = Some(AppState {
            last_change_count,
            pasteboard,
            window,
            icon_label,
            label,
            hide_timer: ptr::null_mut(),
            fade_timer: ptr::null_mut(),
            fade_ticks_elapsed: 0,
            fade_total_ticks: 0,
            settings,
        });

        let _: *mut AnyObject = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: settings.poll_interval_secs
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

        let Some(text) = crate::objc_helpers::nsstring_to_string(raw_text) else {
            return;
        };

        let truncated = truncate_text(
            &text,
            state.settings.truncate_max_width,
            state.settings.truncate_max_lines,
        );
        let message = nsstring_from_str(&truncated);
        let () = msg_send![state.label, setStringValue: message];
        let () = msg_send![message, release];

        layout_hud(state.window, state.icon_label, state.label, state.settings);

        // フェード中なら止めてアルファを戻す
        if !state.fade_timer.is_null() {
            let () = msg_send![state.fade_timer, invalidate];
            state.fade_timer = ptr::null_mut();
        }
        let () = msg_send![state.window, setAlphaValue: 1.0f64];

        let () = msg_send![state.window, orderFrontRegardless];

        if !state.hide_timer.is_null() {
            let () = msg_send![state.hide_timer, invalidate];
        }

        let hide_timer: *mut AnyObject = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: state.settings.hud_duration_secs
            target: this
            selector: sel!(hideHud:)
            userInfo: ptr::null_mut::<AnyObject>()
            repeats: false
        ];
        state.hide_timer = hide_timer;
    }
}

extern "C" fn hide_hud(this: &AnyObject, _: Sel, _: *mut AnyObject) {
    unsafe {
        let mut guard = APP_STATE.lock().expect("APP_STATE lock poisoned");
        let Some(state) = guard.as_mut() else {
            return;
        };

        if !state.hide_timer.is_null() {
            let () = msg_send![state.hide_timer, invalidate];
            state.hide_timer = ptr::null_mut();
        }

        let fade_duration = state.settings.hud_fade_duration_secs;
        if fade_duration <= 0.0 {
            // フェードなし: 即時非表示
            if !state.fade_timer.is_null() {
                let () = msg_send![state.fade_timer, invalidate];
                state.fade_timer = ptr::null_mut();
            }
            let () = msg_send![state.window, orderOut: ptr::null_mut::<AnyObject>()];
            return;
        }

        // フェードアウト開始
        let total_fade_ticks = (fade_duration / FADE_TICK_INTERVAL_SECS).ceil() as u32;
        state.fade_total_ticks = total_fade_ticks;
        if !state.fade_timer.is_null() {
            let () = msg_send![state.fade_timer, invalidate];
            state.fade_timer = ptr::null_mut();
        }
        state.fade_ticks_elapsed = 0;

        let fade_timer: *mut AnyObject = msg_send![
            class!(NSTimer),
            scheduledTimerWithTimeInterval: FADE_TICK_INTERVAL_SECS
            target: this
            selector: sel!(fadeTick:)
            userInfo: ptr::null_mut::<AnyObject>()
            repeats: true
        ];
        state.fade_timer = fade_timer;
    }
}

extern "C" fn fade_tick(_: &AnyObject, _: Sel, timer: *mut AnyObject) {
    unsafe {
        let mut guard = APP_STATE.lock().expect("APP_STATE lock poisoned");
        let Some(state) = guard.as_mut() else {
            let () = msg_send![timer, invalidate];
            return;
        };

        let window = state.window;
        state.fade_ticks_elapsed += 1;

        if state.fade_ticks_elapsed >= state.fade_total_ticks {
            debug_assert!(!state.fade_timer.is_null());
            let () = msg_send![timer, invalidate];
            state.fade_timer = ptr::null_mut();
            drop(guard);

            let () = msg_send![window, setAlphaValue: 0.0f64];
            let () = msg_send![window, orderOut: ptr::null_mut::<AnyObject>()];
            let () = msg_send![window, setAlphaValue: 1.0f64];
        } else {
            let alpha = 1.0 - (state.fade_ticks_elapsed as f64 / state.fade_total_ticks as f64);
            drop(guard);
            let () = msg_send![window, setAlphaValue: alpha];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::FADE_TICK_INTERVAL_SECS;
    use crate::config::DEFAULT_HUD_FADE_DURATION_SECS;

    #[test]
    fn fade_total_ticks_calculation_is_exact() {
        // fade_duration=DEFAULT_HUD_FADE_DURATION_SECS, FADE_TICK_INTERVAL_SECS=1/60 → 18 ticks
        let total = (DEFAULT_HUD_FADE_DURATION_SECS / FADE_TICK_INTERVAL_SECS).ceil() as u32;
        assert_eq!(total, 18);
    }

    #[test]
    fn fade_alpha_is_positive_at_penultimate_tick() {
        let total: u32 = 18;
        let elapsed: u32 = total - 1;
        let alpha = 1.0 - (elapsed as f64 / total as f64);
        assert!(alpha > 0.0, "alpha should be > 0.0, got {}", alpha);
        assert!(
            (alpha - 1.0 / total as f64).abs() < 1e-10,
            "alpha should be approximately 1/total={}, got {}",
            1.0 / total as f64,
            alpha
        );
    }

    #[test]
    fn fade_alpha_is_half_at_midpoint() {
        let total: u32 = 18;
        let elapsed: u32 = 9;
        let alpha = 1.0 - (elapsed as f64 / total as f64);
        assert!((alpha - 0.5).abs() < 1e-10, "alpha={}", alpha);
    }
}
