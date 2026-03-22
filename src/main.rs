mod app;
mod cli;
mod config;
mod error;
mod hud;
mod objc_helpers;
mod png;
mod text;

fn main() {
    if cli::handle_cli_flags() {
        return;
    }

    unsafe {
        use objc2::runtime::AnyObject;
        use objc2::{class, msg_send};

        let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
        assert!(!app.is_null(), "NSApplication の初期化に失敗しました");
        let _: bool = msg_send![app, setActivationPolicy: 1isize];

        let delegate_class = app::get_delegate_class();
        let delegate: *mut AnyObject = msg_send![delegate_class, new];
        let () = msg_send![app, setDelegate: delegate];
        let () = msg_send![app, run];
    }
}
