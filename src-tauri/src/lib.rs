// 入口与接线:Rust 是唯一事实来源,前端纯视图(ARCHITECTURE §2)。
// - timer.rs:状态机(行为对齐旧 service-worker.js)
// - store.rs:持久化(原子写 store.json)
// - tick 线程每秒驱动:到点结算、托盘标题、state-update 快照广播
// - 副作用(Effect)此阶段只打日志,phase 04 接真实音频/通知

use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

mod audio;
mod store;
mod timer;

use store::Store;
use timer::{Durations, Effect};

struct AppState {
    store: Mutex<Store>,
    durations: Durations,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// phase 02:副作用只进日志,行为链路先跑通;phase 04 换成 rodio/通知真实现
fn run_effects(effects: &[Effect]) {
    for e in effects {
        println!("[effect] {:?}", e);
    }
}

fn set_tray_title(app: &AppHandle, title: Option<String>) {
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app2.tray_by_id("main") {
            let _ = tray.set_title(title.as_deref());
        }
    });
}

/// 每次状态变化后的统一收尾:持久化 + 广播快照 + 刷托盘
fn sync_out(app: &AppHandle, store: &Store, dur: &Durations) {
    if let Err(e) = store.save() {
        eprintln!("store: 保存失败 {e}");
    }
    let now = now_ms();
    let today = timer::today_str();
    let snap = timer::snapshot(&store.data, now, &today, dur);
    let _ = app.emit("state-update", &snap);
    set_tray_title(app, timer::tray_title(&store.data.timer_state, now));
}

/// 锁 store → 执行变更 → 收尾。所有 command 走这一条路
fn with_store<T>(
    app: &AppHandle,
    f: impl FnOnce(&mut store::StoreData, i64, &str, &Durations) -> T,
) -> T {
    let state = app.state::<AppState>();
    let mut guard = state.store.lock().expect("store lock poisoned");
    let now = now_ms();
    let today = timer::today_str();
    let out = f(&mut guard.data, now, &today, &state.durations);
    // 变更后如果恰好到点(如加时 0.1 分钟),下一个 tick 会结算,这里不重复处理
    sync_out(app, &guard, &state.durations);
    out
}

// —— commands(名字对齐 ARCHITECTURE §2 的指令清单)——

#[tauri::command]
fn get_state(app: AppHandle) -> timer::Snapshot {
    let state = app.state::<AppState>();
    let mut guard = state.store.lock().expect("store lock poisoned");
    let now = now_ms();
    let today = timer::today_str();
    // 对齐旧 GET_STATE:先校时补结算(睡眠唤醒后 endTime 已过的情况)
    let effects = timer::handle_phase_end_if_due(&mut guard.data, now, &today, &state.durations);
    if !effects.is_empty() {
        run_effects(&effects);
        sync_out(&app, &guard, &state.durations);
    }
    timer::roll_over_tasks_if_needed(&mut guard.data, &today);
    timer::snapshot(&guard.data, now, &today, &state.durations)
}

#[tauri::command]
fn start(app: AppHandle) {
    with_store(&app, |d, now, _, dur| timer::start_focus(d, now, dur));
}

#[tauri::command]
fn pause(app: AppHandle) {
    with_store(&app, |d, now, _, _| timer::pause(d, now));
}

#[tauri::command]
fn resume(app: AppHandle) {
    with_store(&app, |d, now, _, _| timer::resume(d, now));
}

#[tauri::command]
fn reset(app: AppHandle) {
    with_store(&app, |d, _, _, _| timer::reset(d, false));
}

#[tauri::command]
fn abandon(app: AppHandle) {
    with_store(&app, |d, now, today, _| timer::abandon(d, now, today));
}

#[tauri::command]
fn overtime(app: AppHandle, ms: f64) -> timer::OvertimeOutcome {
    with_store(&app, |d, now, today, _| {
        timer::claim_extra_time(d, ms as i64, now, today)
    })
}

#[tauri::command]
fn update_settings(app: AppHandle, patch: serde_json::Value) {
    with_store(&app, |d, _, _, _| timer::update_settings(d, patch));
}

#[tauri::command]
fn reset_quota(app: AppHandle) {
    with_store(&app, |d, _, today, _| timer::reset_quota(d, today));
}

// —— 任务(前端只发指令,数据变更与持久化全在 Rust)——

#[tauri::command]
fn add_task(app: AppHandle, title: String, category: Option<String>, planned: f64) {
    with_store(&app, |d, now, today, _| {
        timer::add_task(d, today, now, &title, category, planned)
    });
}

#[tauri::command]
fn set_task_done(app: AppHandle, id: serde_json::Value, done: bool) {
    with_store(&app, |d, _, today, _| timer::set_task_done(d, today, &id, done));
}

#[tauri::command]
fn set_current_task(app: AppHandle, id: serde_json::Value) {
    with_store(&app, |d, _, today, _| timer::set_current_task(d, today, &id));
}

#[tauri::command]
fn delete_task(app: AppHandle, id: serde_json::Value) {
    with_store(&app, |d, _, today, _| timer::delete_task(d, today, &id));
}

// —— 窗口/托盘 ——

fn toggle_panel(app: &AppHandle) {
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
    let durations = timer::durations();
    if durations.test_mode {
        println!("[TOMATO_TEST] 测试模式:专注 15s / 休息 30s / 最后一分钟 5s");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .setup(move |app| {
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

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
            .expect("apply_vibrancy 失败");

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

            // 加载持久化状态(schema 见 ARCHITECTURE §4)
            let store_path = app
                .path()
                .app_data_dir()
                .expect("app data dir unavailable")
                .join("store.json");
            let store = Store::load(store_path);
            app.manage(AppState { store: Mutex::new(store), durations });

            // 开发自测钩子:TOMATO_AUTOSTART=1 启动即开始一个专注,
            // 无需点 UI 即可在日志观察完整周期(仅测试用,类比旧 TEST_MODE 惯例)
            if std::env::var("TOMATO_AUTOSTART").map(|v| v == "1").unwrap_or(false) {
                let state = app.state::<AppState>();
                let mut guard = state.store.lock().unwrap();
                let now = now_ms();
                timer::start_focus(&mut guard.data, now, &state.durations);
                println!("[TOMATO_AUTOSTART] 已自动开始专注");
            }

            // tick 线程:每秒驱动到点结算 + 托盘标题 + 快照广播(endTime 制,tick 只是采样)
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                let mut was_active = true; // 首拍强制同步一次(覆盖重启恢复)
                loop {
                    {
                        let state = handle.state::<AppState>();
                        let mut guard = state.store.lock().expect("store lock poisoned");
                        let now = now_ms();
                        let today = timer::today_str();
                        let effects = timer::handle_phase_end_if_due(
                            &mut guard.data, now, &today, &state.durations,
                        );
                        let active = guard.data.timer_state.state != store::TimerPhase::Idle;
                        if !effects.is_empty() {
                            run_effects(&effects);
                            sync_out(&handle, &guard, &state.durations);
                        } else if active || was_active {
                            // 倒计时进行中每秒刷新;从 active → idle 的那一拍也要刷一次
                            let snap = timer::snapshot(&guard.data, now, &today, &state.durations);
                            let _ = handle.emit("state-update", &snap);
                            set_tray_title(
                                &handle,
                                timer::tray_title(&guard.data.timer_state, now),
                            );
                        }
                        was_active = active;
                    }
                    std::thread::sleep(Duration::from_secs(1));
                }
            });

            Ok(())
        })
        .on_window_event(|window, event| {
            if window.label() == "panel" {
                if let WindowEvent::Focused(false) = event {
                    let _ = window.hide();
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            start,
            pause,
            resume,
            reset,
            abandon,
            overtime,
            update_settings,
            reset_quota,
            add_task,
            set_task_done,
            set_current_task,
            delete_task,
            audio::play_test_noise,
            audio::play_test_chime
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
