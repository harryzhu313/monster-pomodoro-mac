// Phase 01 spike:托盘 + 面板窗口 + vibrancy + 每秒刷新标题。
// 状态机(timer.rs)、持久化(store.rs)等模块在 phase 02+ 进入,当前的假倒计时仅为功耗 spike。

use std::time::{Duration, Instant};

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

mod audio;

/// 点托盘图标:显示(定位到托盘下方)/ 隐藏主面板
fn toggle_panel(app: &tauri::AppHandle) {
    if let Some(panel) = app.get_webview_window("panel") {
        if panel.is_visible().unwrap_or(false) {
            let _ = panel.hide();
        } else {
            let _ = panel.as_ref().window().move_window(Position::TrayBottomCenter);
            let _ = panel.show();
            let _ = panel.set_focus();
        }
    }
}

pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .setup(|app| {
            // 菜单栏应用:不出现在 Dock
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            // Spike 1:主面板 vibrancy(透明 + 无边框 + NSVisualEffectView 组合)
            let panel = app
                .get_webview_window("panel")
                .expect("panel window declared in tauri.conf.json");
            #[cfg(target_os = "macos")]
            window_vibrancy::apply_vibrancy(
                &panel,
                window_vibrancy::NSVisualEffectMaterial::Popover,
                None,
                Some(12.0),
            )
            .expect("apply_vibrancy 失败(Spike 1 no-go 信号)");

            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?)
                .icon_as_template(true)
                .tooltip("Tomato Monster · 按时停下来")
                .on_tray_icon_event(|tray, event| {
                    tauri_plugin_positioner::on_tray_event(tray.app_handle(), &event);
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        toggle_panel(tray.app_handle());
                    }
                })
                .build(app)?;

            // Spike 2:每秒刷新托盘标题的功耗验证。25:00 假倒计时循环,
            // 真实计时(endTime 制)在 phase 02 由 timer.rs 接管。
            let handle = app.handle().clone();
            let start = Instant::now();
            std::thread::spawn(move || loop {
                let elapsed = start.elapsed().as_secs();
                let remain = 25 * 60 - (elapsed % (25 * 60));
                let title = format!("{:02}:{:02}", remain / 60, remain % 60);
                let h = handle.clone();
                let _ = handle.run_on_main_thread(move || {
                    if let Some(tray) = h.tray_by_id("main") {
                        let _ = tray.set_title(Some(&title));
                    }
                });
                std::thread::sleep(Duration::from_secs(1));
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            // 主面板失焦即隐
            if window.label() == "panel" {
                if let WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            audio::play_test_noise,
            audio::play_test_chime
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
