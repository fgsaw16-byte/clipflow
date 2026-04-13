use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    LogicalSize, Manager, Size, WebviewWindow,
};

#[cfg(target_os = "windows")]
use windows::Win32::{Foundation::POINT, UI::WindowsAndMessaging::GetCursorPos};

pub fn position_window(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen_size = monitor.size();
        let scale_factor = monitor.scale_factor();
        let logical_width = 380.0;
        let logical_height = 430.0;
        let physical_width = (logical_width * scale_factor) as i32;
        let physical_height = (logical_height * scale_factor) as i32;
        let margin = (20.0 * scale_factor) as i32;
        let taskbar_allowance = (40.0 * scale_factor) as i32;
        let x = screen_size.width as i32 - physical_width - margin;
        let y = screen_size.height as i32 - physical_height - taskbar_allowance;
        let _ = window.set_size(Size::Logical(LogicalSize {
            width: logical_width,
            height: logical_height,
        }));
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
}

pub fn position_window_at_mouse(window: &WebviewWindow) {
    if let Ok(Some(monitor)) = window.current_monitor() {
        let screen_size = monitor.size();
        let scale_factor = monitor.scale_factor();

        let logical_width = 380.0;
        let logical_height = 430.0;
        let physical_width = (logical_width * scale_factor) as i32;
        let physical_height = (logical_height * scale_factor) as i32;

        #[cfg(target_os = "windows")]
        let (mx, my) = {
            let mut pt = POINT { x: 0, y: 0 };
            unsafe {
                if GetCursorPos(&mut pt).is_ok() {
                    (pt.x, pt.y)
                } else {
                    position_window(window);
                    return;
                }
            }
        };

        #[cfg(not(target_os = "windows"))]
        let (mx, my) = {
            position_window(window);
            return;
        };

        let anchor_x = (120.0 * scale_factor) as i32;
        let anchor_y = (78.0 * scale_factor) as i32;
        let mut x = mx - anchor_x;
        let mut y = my - anchor_y;

        let max_x = screen_size.width as i32 - physical_width;
        let max_y = screen_size.height as i32 - physical_height;
        if x < 0 {
            x = 0;
        }
        if y < 0 {
            y = 0;
        }
        if x > max_x {
            x = max_x;
        }
        if y > max_y {
            y = max_y;
        }

        let _ = window.set_size(Size::Logical(LogicalSize {
            width: logical_width,
            height: logical_height,
        }));
        let _ = window.set_position(tauri::Position::Physical(tauri::PhysicalPosition { x, y }));
    }
}

pub fn build_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let quit_i = MenuItem::with_id(app, "quit", "退出 ClipFlow", true, None::<&str>)?;
    let show_i = MenuItem::with_id(app, "show", "显示主界面", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&show_i, &quit_i])?;
    let _tray = TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .tooltip("ClipFlow")
        .menu(&menu)
        .on_menu_event(|app, event| match event.id().as_ref() {
            "quit" => app.exit(0),
            "show" => {
                if let Some(w) = app.get_webview_window("main") {
                    position_window(&w);
                    let _ = w.show();
                    let _ = w.set_focus();
                }
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(w) = app.get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        position_window(&w);
                        let _ = w.show();
                        let _ = w.set_focus();
                    }
                }
            }
        })
        .build(app)?;
    Ok(())
}
