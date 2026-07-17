// 入口与接线:Rust 是唯一事实来源,前端纯视图(ARCHITECTURE §2)。
// - timer.rs:状态机(行为对齐旧 service-worker.js)
// - store.rs:持久化(原子写 store.json)
// - tick 线程每秒驱动:到点结算、托盘标题、state-update 快照广播
// - 副作用(Effect)此阶段只打日志,phase 04 接真实音频/通知

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::image::Image;
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, WindowEvent};
use tauri_plugin_positioner::{Position, WindowExt};

mod audio;
mod store;
mod timer;

use audio::{AudioCmd, AudioEngine};
use store::Store;
use timer::{Durations, Effect};

struct AppState {
    store: Mutex<Store>,
    durations: Durations,
    audio: AudioEngine,
    /// 最后一分钟提示的去重标记(按 endTime,内存态)
    last_minute_fired: Mutex<Option<i64>>,
    /// 休息期面板钉住:失焦不隐藏、托盘点击不收起(blur 回调里免锁读取)
    panel_pinned: AtomicBool,
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

/// 效果执行器:设置开关在这里裁决,音频引擎与系统通知是纯执行方
fn run_effects(app: &AppHandle, settings: &store::Settings, effects: &[Effect]) {
    let state = app.state::<AppState>();
    for e in effects {
        println!("[effect] {:?}", e);
        match e {
            Effect::Chime => {
                if settings.chime_enabled {
                    state.audio.send(AudioCmd::Chime);
                }
            }
            // 开关(lastMinuteEnabled)在生成处 check_last_minute 已裁决
            Effect::SoftNudge => state.audio.send(AudioCmd::SoftNudge),
            Effect::WhiteNoiseStart => {
                if settings.white_noise_enabled {
                    // 延迟 1 秒起播,与 chime 错开(旧 offscreen 行为)
                    state.audio.send(AudioCmd::NoiseStart {
                        delay: std::time::Duration::from_millis(1000),
                    });
                }
            }
            Effect::WhiteNoiseStop => state.audio.send(AudioCmd::NoiseStop),
            Effect::PanelPin => {
                state.panel_pinned.store(true, Ordering::SeqCst);
                show_panel(app, false); // 不抢键盘焦点(用户拍板)
            }
            Effect::PanelUnpin => {
                state.panel_pinned.store(false, Ordering::SeqCst);
                if let Some(panel) = app.get_webview_window("panel") {
                    let _ = panel.hide();
                }
            }
            // phase 07 接 love monster 全屏窗口
            Effect::Celebration => {}
        }
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
        run_effects(&app, &guard.data.settings.clone(), &effects);
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
    let outcome = with_store(&app, |d, now, today, _| {
        timer::claim_extra_time(d, ms as i64, now, today)
    });
    if outcome.ok {
        // 加时成功 = 选择继续工作:解除钉住、收起面板、停白噪音
        let state = app.state::<AppState>();
        state.panel_pinned.store(false, Ordering::SeqCst);
        state.audio.send(AudioCmd::NoiseStop);
        if let Some(panel) = app.get_webview_window("panel") {
            let _ = panel.hide();
        }
    }
    outcome
}

#[tauri::command]
fn update_settings(app: AppHandle, patch: serde_json::Value) {
    let (noise_on, breaking) = with_store(&app, |d, _, _, _| {
        timer::update_settings(d, patch);
        (
            d.settings.white_noise_enabled,
            d.timer_state.state == store::TimerPhase::Breaking,
        )
    });
    // 白噪音状态驱动同步(等价旧 syncWhiteNoise):休息中开关立即生效
    let state = app.state::<AppState>();
    if !noise_on {
        state.audio.send(AudioCmd::NoiseStop);
    } else if breaking {
        state.audio.send(AudioCmd::NoiseStart { delay: std::time::Duration::ZERO });
    }
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

/// 显示面板;focus=false 时不抢键盘焦点(休息自动弹出用)
fn show_panel(app: &AppHandle, focus: bool) {
    if let Some(panel) = app.get_webview_window("panel") {
        // 托盘位置未知时(如刚启动未碰过托盘)定位会失败,退回右上角
        if panel.as_ref().window().move_window(Position::TrayBottomCenter).is_err() {
            let _ = panel.as_ref().window().move_window(Position::TopRight);
        }
        let _ = panel.show();
        if focus {
            let _ = panel.set_focus();
        }
    }
}

fn toggle_panel(app: &AppHandle) {
    let pinned = app.state::<AppState>().panel_pinned.load(Ordering::SeqCst);
    if let Some(panel) = app.get_webview_window("panel") {
        if panel.is_visible().unwrap_or(false) {
            // 钉住期间(休息中)点托盘不收起——无免费跳过的一部分
            if !pinned {
                let _ = panel.hide();
            }
        } else {
            show_panel(app, true);
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
            // 重启恢复到休息中:白噪音接着放、面板重新钉住(状态驱动)
            let resume_break = store.data.timer_state.state == store::TimerPhase::Breaking;
            let resume_noise = resume_break && store.data.settings.white_noise_enabled;
            app.manage(AppState {
                store: Mutex::new(store),
                durations,
                audio: AudioEngine::new(),
                last_minute_fired: Mutex::new(None),
                panel_pinned: AtomicBool::new(false),
            });
            let state = app.state::<AppState>();
            if resume_noise {
                state.audio.send(AudioCmd::NoiseStart { delay: std::time::Duration::ZERO });
            }
            if resume_break {
                state.panel_pinned.store(true, Ordering::SeqCst);
                show_panel(app.handle(), false);
            }

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
                        let mut effects = timer::handle_phase_end_if_due(
                            &mut guard.data, now, &today, &state.durations,
                        );
                        {
                            let mut fired =
                                state.last_minute_fired.lock().expect("lock poisoned");
                            effects.extend(timer::check_last_minute(
                                &guard.data, now, &state.durations, &mut fired,
                            ));
                        }
                        let active = guard.data.timer_state.state != store::TimerPhase::Idle;
                        if !effects.is_empty() {
                            run_effects(&handle, &guard.data.settings.clone(), &effects);
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
                    // 失焦即隐;休息期钉住时例外(点面板外不收起)
                    let pinned = window
                        .app_handle()
                        .state::<AppState>()
                        .panel_pinned
                        .load(Ordering::SeqCst);
                    if !pinned {
                        let _ = window.hide();
                    }
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
            delete_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
