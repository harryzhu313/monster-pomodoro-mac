// 入口与接线:Rust 是本地业务状态唯一事实来源,前端是视图与受限 Notion 适配层(ARCHITECTURE §2)。
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

const PRODUCTION_IDENTIFIER: &str = "com.tomatomonster.app";

/// Debug 进程必须使用开发版 identifier，避免误读写正式版 store。
fn validate_development_identity(identifier: &str, debug_build: bool) -> std::io::Result<()> {
    if debug_build && identifier == PRODUCTION_IDENTIFIER {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "debug 启动被拒绝：请使用 `cargo tauri dev --config src-tauri/tauri.dev.conf.json`，避免读写正式版数据",
        ));
    }
    Ok(())
}

#[cfg(test)]
mod development_identity_tests {
    use super::validate_development_identity;

    #[test]
    fn debug_build_rejects_production_identifier() {
        assert!(validate_development_identity("com.tomatomonster.app", true).is_err());
    }

    #[test]
    fn debug_build_accepts_development_identifier() {
        assert!(validate_development_identity("com.tomatomonster.dev", true).is_ok());
    }

    #[test]
    fn release_build_accepts_production_identifier() {
        assert!(validate_development_identity("com.tomatomonster.app", false).is_ok());
    }
}

struct AppState {
    store: Mutex<Store>,
    durations: Durations,
    audio: AudioEngine,
    /// 最后一分钟提示的去重标记(按 endTime,内存态)
    last_minute_fired: Mutex<Option<i64>>,
    /// 休息期面板钉住:失焦不隐藏、托盘点击不收起(blur 回调里免锁读取)
    panel_pinned: AtomicBool,
    /// 托盘图标当前状态类别(只在变化时 set_icon,避免每秒重设)
    last_tray_phase: Mutex<Option<store::TimerPhase>>,
    /// 面板/统计窗口可见性(tick 只在有人看时广播快照——CPU 预算 §9)
    panel_visible: AtomicBool,
    stats_open: AtomicBool,
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
                // 不抢键盘焦点(用户拍板);此处仍持有调用方的 store 锁,
                // 用不广播的窗口操作版本——紧随其后的 sync_out 会广播最新快照
                show_panel_window(app, false);
            }
            Effect::PanelUnpin => {
                state.panel_pinned.store(false, Ordering::SeqCst);
                state.panel_visible.store(false, Ordering::SeqCst);
                if let Some(panel) = app.get_webview_window("panel") {
                    let _ = panel.hide();
                }
            }
            Effect::Celebration => open_celebration(app),
        }
    }
}

/// 托盘三态模板剪影(CONTENT.md 菜单栏节:idle=happy / 专注=calm / 休息=angry)
fn tray_icon_bytes(phase: store::TimerPhase) -> &'static [u8] {
    match phase {
        store::TimerPhase::Idle => include_bytes!("../icons/tray-idle.png"),
        // 暂停归专注语境,用 calm
        store::TimerPhase::Focusing | store::TimerPhase::Paused => {
            include_bytes!("../icons/tray-focus.png")
        }
        store::TimerPhase::Breaking => include_bytes!("../icons/tray-break.png"),
    }
}

fn set_tray_title(app: &AppHandle, title: Option<String>, phase: store::TimerPhase) {
    let app2 = app.clone();
    let state = app.state::<AppState>();
    // 图标只在状态类别变化时更换,标题每秒都刷
    let icon_changed = {
        let mut last = state.last_tray_phase.lock().expect("lock poisoned");
        if *last != Some(phase) {
            *last = Some(phase);
            true
        } else {
            false
        }
    };
    let _ = app.run_on_main_thread(move || {
        if let Some(tray) = app2.tray_by_id("main") {
            let _ = tray.set_title(title.as_deref());
            if icon_changed {
                if let Ok(img) = Image::from_bytes(tray_icon_bytes(phase)) {
                    let _ = tray.set_icon(Some(img));
                    let _ = tray.set_icon_as_template(true);
                }
            }
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
    set_tray_title(
        app,
        timer::tray_title(&store.data.timer_state, now),
        store.data.timer_state.state,
    );
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
async fn get_state(app: AppHandle) -> timer::Snapshot {
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
async fn start(app: AppHandle) {
    with_store(&app, |d, now, _, dur| timer::start_focus(d, now, dur));
}

#[tauri::command]
async fn pause(app: AppHandle) {
    with_store(&app, |d, now, _, _| timer::pause(d, now));
}

#[tauri::command]
async fn resume(app: AppHandle) {
    with_store(&app, |d, now, _, _| timer::resume(d, now));
}

#[tauri::command]
async fn reset(app: AppHandle) {
    with_store(&app, |d, _, _, _| timer::reset(d, false));
}

#[tauri::command]
async fn abandon(app: AppHandle) {
    with_store(&app, |d, now, today, _| timer::abandon(d, now, today));
}

#[tauri::command]
async fn overtime(app: AppHandle, ms: f64) -> timer::OvertimeOutcome {
    let outcome = with_store(&app, |d, now, today, _| {
        timer::claim_extra_time(d, ms as i64, now, today)
    });
    if outcome.ok {
        // 加时成功 = 选择继续工作:解除钉住、收起面板、停白噪音
        let state = app.state::<AppState>();
        state.panel_pinned.store(false, Ordering::SeqCst);
        state.panel_visible.store(false, Ordering::SeqCst);
        state.audio.send(AudioCmd::NoiseStop);
        if let Some(panel) = app.get_webview_window("panel") {
            let _ = panel.hide();
        }
    }
    outcome
}

#[tauri::command]
async fn update_settings(app: AppHandle, patch: serde_json::Value) {
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
        state.audio.send(AudioCmd::NoiseStart {
            delay: std::time::Duration::ZERO,
        });
    }
}

#[tauri::command]
async fn reset_quota(app: AppHandle) {
    with_store(&app, |d, _, today, _| timer::reset_quota(d, today));
}

// —— 任务(前端只发指令,数据变更与持久化全在 Rust)——

#[tauri::command]
async fn add_task(app: AppHandle, title: String, category: Option<String>, planned: f64) {
    with_store(&app, |d, now, today, _| {
        timer::add_task(d, today, now, &title, category, planned)
    });
}

#[tauri::command]
async fn set_task_done(app: AppHandle, id: serde_json::Value, done: bool) {
    with_store(&app, |d, _, today, _| {
        timer::set_task_done(d, today, &id, done)
    });
}

#[tauri::command]
async fn set_current_task(app: AppHandle, id: serde_json::Value) {
    with_store(&app, |d, _, today, _| {
        timer::set_current_task(d, today, &id)
    });
}

#[tauri::command]
async fn delete_task(app: AppHandle, id: serde_json::Value) {
    with_store(&app, |d, _, today, _| timer::delete_task(d, today, &id));
}

/// 改今天这条任务的计划番茄数;下限夹紧在 Rust 侧,前端的 min 只是体验优化
#[tauri::command]
async fn set_task_planned(app: AppHandle, id: serde_json::Value, planned: f64) {
    with_store(&app, |d, _, today, _| {
        timer::set_task_planned(d, today, &id, planned)
    });
}

// —— 月度目标(v2)——

#[tauri::command]
async fn add_goal(app: AppHandle, title: String, category: Option<String>, month_target: f64) {
    with_store(&app, |d, now, _, _| {
        timer::add_goal(d, now, &title, category, month_target)
    });
}

#[tauri::command]
async fn update_goal(
    app: AppHandle,
    id: String,
    title: Option<String>,
    category: Option<String>,
    month_target: Option<f64>,
) {
    with_store(&app, |d, _, _, _| {
        timer::update_goal(d, &id, title, category, month_target)
    });
}

#[tauri::command]
async fn delete_goal(app: AppHandle, id: String) {
    with_store(&app, |d, _, _, _| timer::delete_goal(d, &id));
}

#[tauri::command]
async fn toggle_goal_today(app: AppHandle, id: String) -> timer::GoalToggleOutcome {
    with_store(&app, |d, now, today, _| {
        timer::toggle_goal_today(d, today, now, &id)
    })
}

// —— 备份导入导出 / Notion 配置 ——

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ExportOutcome {
    ok: bool,
    canceled: bool,
    path: Option<String>,
    error: Option<String>,
}

fn write_backup_file(path: &std::path::Path, json: &str) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::fs::{OpenOptions, Permissions};
        use std::io::Write;
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

        // 不在 open 时 truncate：先确认能收窄权限，失败时不破坏同名旧备份。
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .mode(0o600)
            .open(path)?;
        file.set_permissions(Permissions::from_mode(0o600))?;
        file.set_len(0)?;
        file.write_all(json.as_bytes())?;
        return file.sync_all();
    }

    #[cfg(not(unix))]
    std::fs::write(path, json)
}

#[tauri::command]
async fn export_backup(app: AppHandle) -> ExportOutcome {
    use tauri_plugin_dialog::DialogExt;
    let (json, file_name) = {
        let state = app.state::<AppState>();
        let guard = state.store.lock().expect("store lock poisoned");
        (
            store::export_backup_json(&guard.data, env!("CARGO_PKG_VERSION")),
            format!("tomato-monster-backup-{}.json", timer::today_str()),
        )
    };
    let Some(path) = app
        .dialog()
        .file()
        .set_file_name(&file_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file()
    else {
        return ExportOutcome {
            ok: false,
            canceled: true,
            path: None,
            error: None,
        };
    };
    let Ok(path) = path.into_path() else {
        return ExportOutcome {
            ok: false,
            canceled: false,
            path: None,
            error: Some("无效的保存路径".into()),
        };
    };
    match write_backup_file(&path, &json) {
        Ok(()) => ExportOutcome {
            ok: true,
            canceled: false,
            path: Some(path.display().to_string()),
            error: None,
        },
        Err(e) => ExportOutcome {
            ok: false,
            canceled: false,
            path: None,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
async fn import_backup(app: AppHandle, json: String) -> store::ImportOutcome {
    with_store(&app, |d, _, _, _| match store::import_backup(d, &json) {
        Ok(days) => store::ImportOutcome {
            ok: true,
            error: None,
            days,
        },
        Err(e) => store::ImportOutcome {
            ok: false,
            error: Some(e),
            days: 0,
        },
    })
}

#[tauri::command]
async fn get_notion_config(app: AppHandle) -> store::NotionConfig {
    let state = app.state::<AppState>();
    let guard = state.store.lock().expect("store lock poisoned");
    guard.data.notion_config.clone()
}

#[tauri::command]
async fn set_notion_config(app: AppHandle, config: store::NotionConfigPatch) -> Result<(), String> {
    let state = app.state::<AppState>();
    let mut guard = state.store.lock().expect("store lock poisoned");
    guard
        .update_notion_config(config)
        .map_err(|e| e.to_string())
}

fn notion_export_entry(date: &str, at: String, summary: serde_json::Value) -> serde_json::Value {
    let mut entry = serde_json::Map::new();
    entry.insert("at".into(), serde_json::Value::String(at));
    if let serde_json::Value::Object(m) = summary {
        for (k, v) in m {
            if k != "at" {
                entry.insert(k, v);
            }
        }
    }
    // Rust 边界再次执行统一白名单，防止未来前端误传的敏感字段进入 store/备份。
    store::sanitize_notion_export_entry(date, &serde_json::Value::Object(entry))
}

/// 记一笔 Notion 导出历史(旧 notionExportLog 语义:prev[date] = {at, ...summary})
#[tauri::command]
async fn log_notion_export(app: AppHandle, date: String, summary: serde_json::Value) {
    with_store(&app, |d, _, _, _| {
        let entry = notion_export_entry(&date, chrono::Local::now().to_rfc3339(), summary);
        d.notion_export_log.insert(date, entry);
    });
}

#[cfg(test)]
mod notion_export_tests {
    use super::{notion_export_entry, write_backup_file};
    use serde_json::json;

    #[test]
    fn monthly_export_log_keeps_only_timestamp_and_counts() {
        let entry = notion_export_entry(
            "monthly:2026-08",
            "2026-08-25T10:00:00+08:00".into(),
            json!({
                "created": 2,
                "updated": 3,
                "failed": 1,
                "token": "should-not-persist",
                "databaseId": "should-not-persist",
                "request": { "properties": "should-not-persist" },
                "errors": [{ "title": "should-not-persist" }]
            }),
        );
        assert_eq!(
            entry,
            json!({
                "at": "2026-08-25T10:00:00+08:00",
                "created": 2,
                "updated": 3,
                "failed": 1
            })
        );
        assert!(!entry.to_string().contains("should-not-persist"));
    }

    #[test]
    fn daily_export_log_keeps_safe_feedback_fields_only() {
        let summary = json!({
            "ok": false,
            "created": 1,
            "updated": 0,
            "failed": 1,
            "total": 2,
            "existingDuplicates": 3,
            "dayPageLinked": true,
            "errors": [{ "task": "should-not-persist", "error": "should-not-persist" }],
            "token": "should-not-persist"
        });
        let entry = notion_export_entry(
            "2026-08-25",
            "2026-08-25T10:00:00+08:00".into(),
            summary.clone(),
        );
        assert_eq!(entry["ok"], summary["ok"]);
        assert_eq!(entry["created"], 1);
        assert_eq!(entry["updated"], 0);
        assert_eq!(entry["failed"], 1);
        assert_eq!(entry["total"], 2);
        assert_eq!(entry["existingDuplicates"], 3);
        assert_eq!(entry["dayPageLinked"], true);
        assert_eq!(entry["at"], "2026-08-25T10:00:00+08:00");
        assert!(entry.get("errors").is_none());
        assert!(!entry.to_string().contains("should-not-persist"));
    }

    #[cfg(unix)]
    #[test]
    fn backup_file_is_written_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let unique = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "tomato-monster-private-backup-{}-{unique}.json",
            std::process::id()
        ));
        write_backup_file(&path, r#"{"fixture":true}"#).unwrap();
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            r#"{"fixture":true}"#
        );
        let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
        std::fs::remove_file(path).unwrap();
    }
}

// —— 全局快捷键 / 开机自启 / 首启引导 / 庆祝弹层 ——

/// 快捷键动作:开始/暂停/继续;休息中无操作(红线 1:快捷键不提供跳过出口)
fn shortcut_toggle(app: &AppHandle) {
    with_store(app, |d, now, _, dur| match d.timer_state.state {
        store::TimerPhase::Idle => timer::start_focus(d, now, dur),
        store::TimerPhase::Focusing => timer::pause(d, now),
        store::TimerPhase::Paused => timer::resume(d, now),
        store::TimerPhase::Breaking => {}
    });
}

/// 注册全局快捷键;accel 空串 = 关闭。冲突等注册失败返回 Err 供 UI 降级提示
fn apply_global_shortcut(app: &AppHandle, accel: &str) -> Result<(), String> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;
    let gs = app.global_shortcut();
    let _ = gs.unregister_all();
    if accel.is_empty() {
        return Ok(());
    }
    gs.register(accel).map_err(|e| e.to_string())
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ShortcutOutcome {
    ok: bool,
    error: Option<String>,
}

#[tauri::command]
async fn set_global_shortcut(app: AppHandle, accel: String) -> ShortcutOutcome {
    match apply_global_shortcut(&app, &accel) {
        Ok(()) => {
            with_store(&app, |d, _, _, _| d.settings.global_shortcut = accel);
            ShortcutOutcome {
                ok: true,
                error: None,
            }
        }
        Err(e) => {
            // 注册失败:回滚到原有快捷键,不写入设置
            let prev = {
                let state = app.state::<AppState>();
                let guard = state.store.lock().expect("store lock poisoned");
                guard.data.settings.global_shortcut.clone()
            };
            let _ = apply_global_shortcut(&app, &prev);
            ShortcutOutcome {
                ok: false,
                error: Some(e),
            }
        }
    }
}

#[tauri::command]
async fn get_autostart(app: AppHandle) -> bool {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
async fn set_autostart(app: AppHandle, enabled: bool) -> ShortcutOutcome {
    use tauri_plugin_autostart::ManagerExt;
    let r = if enabled {
        app.autolaunch().enable()
    } else {
        app.autolaunch().disable()
    };
    match r {
        Ok(()) => ShortcutOutcome {
            ok: true,
            error: None,
        },
        Err(e) => ShortcutOutcome {
            ok: false,
            error: Some(e.to_string()),
        },
    }
}

#[tauri::command]
async fn finish_onboarding(app: AppHandle) {
    with_store(&app, |d, _, _, _| d.onboarding_done = true);
}

/// love monster 全屏庆祝窗口(临时窗口,展示完销毁——ARCHITECTURE §3)
fn open_celebration(app: &AppHandle) {
    if app.get_webview_window("celebration").is_some() {
        return;
    }
    let mut builder = tauri::WebviewWindowBuilder::new(
        app,
        "celebration",
        tauri::WebviewUrl::App("celebration/celebration.html".into()),
    )
    .decorations(false)
    .transparent(true)
    .always_on_top(true)
    .skip_taskbar(true)
    .visible_on_all_workspaces(true)
    .focused(true);
    if let Ok(Some(mon)) = app.primary_monitor() {
        let scale = mon.scale_factor();
        let size = mon.size();
        let pos = mon.position();
        builder = builder
            .inner_size(size.width as f64 / scale, size.height as f64 / scale)
            .position(pos.x as f64 / scale, pos.y as f64 / scale);
    }
    if let Err(e) = builder.build() {
        eprintln!("celebration 窗口创建失败: {e}");
    }
}

#[tauri::command]
async fn close_celebration(app: AppHandle) {
    if let Some(w) = app.get_webview_window("celebration") {
        let _ = w.destroy();
    }
}

// —— 统计与设置窗口 ——

#[tauri::command]
async fn open_stats(app: AppHandle) {
    if let Some(w) = app.get_webview_window("stats") {
        let _ = w.set_focus();
        return;
    }
    // 按需创建,关闭销毁(ARCHITECTURE §3)
    match tauri::WebviewWindowBuilder::new(
        &app,
        "stats",
        tauri::WebviewUrl::App("stats/stats.html".into()),
    )
    .title("Tomato Monster · 统计与设置")
    .inner_size(680.0, 760.0)
    .min_inner_size(560.0, 480.0)
    .transparent(true)
    .build()
    {
        Ok(w) => {
            app.state::<AppState>()
                .stats_open
                .store(true, Ordering::SeqCst);
            // Sidebar 材质磨砂感强;UnderWindowBackground 太透,壁纸会穿透(用户反馈)
            #[cfg(target_os = "macos")]
            if let Err(e) = window_vibrancy::apply_vibrancy(
                &w,
                window_vibrancy::NSVisualEffectMaterial::Sidebar,
                None,
                None,
            ) {
                eprintln!("stats vibrancy 失败: {e}");
            }
        }
        Err(e) => eprintln!("stats 窗口创建失败: {e}"),
    }
}

#[tauri::command]
async fn get_stats_bundle(app: AppHandle) -> timer::StatsBundle {
    let state = app.state::<AppState>();
    let mut guard = state.store.lock().expect("store lock poisoned");
    let today = timer::today_str();
    timer::roll_over_tasks_if_needed(&mut guard.data, &today);
    timer::stats_bundle(&guard.data, &today)
}

#[tauri::command]
async fn clear_today_stats(app: AppHandle) {
    with_store(&app, |d, _, today, _| timer::clear_today_stats(d, today));
}

#[tauri::command]
async fn set_history_task_done(app: AppHandle, date: String, id: serde_json::Value, done: bool) {
    with_store(&app, |d, _, today, _| {
        timer::set_history_task_done(d, today, &date, &id, done)
    });
}

// —— 窗口/托盘 ——

/// 显示面板;focus=false 时不抢键盘焦点(休息自动弹出用)。
/// 只做窗口操作,不广播快照——调用方如果没有在锁内(effect 执行器以外的独立调用点),
/// 用下面的 show_panel 补一拍;effect 执行器内已持有 store 锁,广播交给紧随其后的 sync_out,
/// 避免在锁内重入 emit_fresh_snapshot 造成同线程自死锁(实际使用中每次专注结束必现)。
fn show_panel_window(app: &AppHandle, focus: bool) {
    if let Some(panel) = app.get_webview_window("panel") {
        // 托盘位置未知时(如刚启动未碰过托盘)定位会失败,退回右上角
        if panel
            .as_ref()
            .window()
            .move_window(Position::TrayBottomCenter)
            .is_err()
        {
            let _ = panel.as_ref().window().move_window(Position::TopRight);
        }
        let _ = panel.show();
        if focus {
            let _ = panel.set_focus();
        }
        app.state::<AppState>()
            .panel_visible
            .store(true, Ordering::SeqCst);
    }
}

/// show_panel_window + 广播一拍最新快照;非 effect 路径的独立调用点(启动恢复、托盘点击)用这个
fn show_panel(app: &AppHandle, focus: bool) {
    show_panel_window(app, focus);
    emit_fresh_snapshot(app); // 隐藏期间不广播,显示瞬间补一拍最新状态
}

/// 只广播不落盘:窗口从隐藏转可见时刷新其显示
fn emit_fresh_snapshot(app: &AppHandle) {
    let state = app.state::<AppState>();
    let mut guard = state.store.lock().expect("store lock poisoned");
    let now = now_ms();
    let today = timer::today_str();
    // 面板隐藏期间可能已跨天:显示前先归档,不让昨天的任务闪出来
    if timer::roll_over_tasks_if_needed(&mut guard.data, &today) {
        if let Err(e) = guard.save() {
            eprintln!("store: 保存失败 {e}");
        }
    }
    let snap = timer::snapshot(&guard.data, now, &today, &state.durations);
    let _ = app.emit("state-update", &snap);
}

fn toggle_panel(app: &AppHandle) {
    let pinned = app.state::<AppState>().panel_pinned.load(Ordering::SeqCst);
    if let Some(panel) = app.get_webview_window("panel") {
        if panel.is_visible().unwrap_or(false) {
            // 钉住期间(休息中)点托盘不收起——无免费跳过的一部分
            if !pinned {
                let _ = panel.hide();
                app.state::<AppState>()
                    .panel_visible
                    .store(false, Ordering::SeqCst);
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

    // 在插件初始化和 app data 路径解析之前失败，确保 debug 不会碰正式版数据。
    let context = tauri::generate_context!();
    validate_development_identity(&context.config().identifier, cfg!(debug_assertions))
        .unwrap_or_else(|message| panic!("{message}"));

    tauri::Builder::default()
        .plugin(tauri_plugin_positioner::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_http::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_handler(|app, _shortcut, event| {
                    // 只注册一个快捷键,按下即触发开始/暂停(休息中无操作)
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        shortcut_toggle(app);
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(move |app| {
            let app_name = app
                .config()
                .product_name
                .clone()
                .unwrap_or_else(|| "Tomato Monster".into());

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

            // 托盘右键菜单:统计与设置 / 退出(左键仍是 toggle 面板)。
            // 退出不构成休息跳过:状态持久化,重开即恢复(红线审计已复核)
            let menu = {
                use tauri::menu::{MenuBuilder, MenuItemBuilder};
                let stats_item = MenuItemBuilder::with_id("stats", "统计与设置").build(app)?;
                let quit_item =
                    MenuItemBuilder::with_id("quit", format!("退出 {app_name}")).build(app)?;
                MenuBuilder::new(app)
                    .item(&stats_item)
                    .separator()
                    .item(&quit_item)
                    .build()?
            };

            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(include_bytes!("../icons/tray-idle.png"))?)
                .icon_as_template(true)
                .tooltip(format!("{app_name} · 按时停下来"))
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "stats" => {
                        tauri::async_runtime::spawn(open_stats(app.clone()));
                    }
                    "quit" => app.exit(0),
                    _ => {}
                })
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
            let store_path = if durations.test_mode {
                // 全链路测试不得给真实统计增加 15 秒测试番茄；按进程隔离到系统临时目录。
                std::env::temp_dir().join(format!(
                    "tomato-monster-test-{}-{}.json",
                    std::process::id(),
                    chrono::Local::now().timestamp_millis()
                ))
            } else {
                app.path()
                    .app_data_dir()
                    .expect("app data dir unavailable")
                    .join("store.json")
            };
            let mut store = Store::load(store_path);
            if durations.test_mode {
                // 测试周期直接进入产品面板，不被首次引导遮挡。
                store.data.onboarding_done = true;
            }
            // 重启恢复到休息中:白噪音接着放、面板重新钉住(状态驱动)
            let resume_break = store.data.timer_state.state == store::TimerPhase::Breaking;
            let resume_noise = resume_break && store.data.settings.white_noise_enabled;
            let shortcut = store.data.settings.global_shortcut.clone();
            let needs_onboarding = !store.data.onboarding_done;
            app.manage(AppState {
                store: Mutex::new(store),
                durations,
                audio: AudioEngine::new(),
                last_minute_fired: Mutex::new(None),
                panel_pinned: AtomicBool::new(false),
                last_tray_phase: Mutex::new(None),
                panel_visible: AtomicBool::new(false),
                stats_open: AtomicBool::new(false),
            });
            let state = app.state::<AppState>();
            if resume_noise {
                state.audio.send(AudioCmd::NoiseStart {
                    delay: std::time::Duration::ZERO,
                });
            }
            if resume_break {
                state.panel_pinned.store(true, Ordering::SeqCst);
                show_panel(app.handle(), false);
            }

            // 全局快捷键:启动时按设置注册;冲突不阻塞启动,设置页可改
            if let Err(e) = apply_global_shortcut(app.handle(), &shortcut) {
                eprintln!("全局快捷键 {shortcut} 注册失败(可能冲突): {e}");
            }

            // 首次启动:自动弹出面板进入引导流(欢迎/导入 → 自启 → 快捷键)
            if needs_onboarding {
                show_panel(app.handle(), true);
            }

            // 开发自测钩子:TOMATO_AUTOSTART=1 启动即开始一个专注,
            // 无需点 UI 即可在日志观察完整周期(仅测试用,类比旧 TEST_MODE 惯例)
            if std::env::var("TOMATO_AUTOSTART")
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                let state = app.state::<AppState>();
                let mut guard = state.store.lock().unwrap();
                let now = now_ms();
                timer::start_focus(&mut guard.data, now, &state.durations);
                println!("[TOMATO_AUTOSTART] 已自动开始专注");
            }

            // 开发自测钩子:启动即开统计窗口(验证窗口创建/vibrancy,仅测试用)
            if std::env::var("TOMATO_OPEN_STATS")
                .map(|v| v == "1")
                .unwrap_or(false)
            {
                tauri::async_runtime::spawn(open_stats(app.handle().clone()));
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
                            &mut guard.data,
                            now,
                            &today,
                            &state.durations,
                        );
                        {
                            let mut fired = state.last_minute_fired.lock().expect("lock poisoned");
                            effects.extend(timer::check_last_minute(
                                &guard.data,
                                now,
                                &state.durations,
                                &mut fired,
                            ));
                        }
                        // 跨天归档:放在结算之后(与 get_state 同序,跨零点的专注先记到
                        // 昨天的任务上再归档);日期没变时是一次字符串比较,零开销
                        let rolled = timer::roll_over_tasks_if_needed(&mut guard.data, &today);
                        let active = guard.data.timer_state.state != store::TimerPhase::Idle;
                        if !effects.is_empty() {
                            run_effects(&handle, &guard.data.settings.clone(), &effects);
                            sync_out(&handle, &guard, &state.durations);
                        } else if rolled {
                            // 归档已经改了数据,必须落盘并广播——否则面板一直显示昨天的任务
                            // (v1 的 bug:tick 不做日切,只有打开统计窗口才顺带切一次)
                            sync_out(&handle, &guard, &state.durations);
                        } else if active || was_active {
                            // 托盘标题每秒都刷;快照只在有窗口在看时广播(CPU 预算 §9)
                            let watching = state.panel_visible.load(Ordering::SeqCst)
                                || state.stats_open.load(Ordering::SeqCst);
                            if watching || !active {
                                let snap =
                                    timer::snapshot(&guard.data, now, &today, &state.durations);
                                let _ = handle.emit("state-update", &snap);
                            }
                            set_tray_title(
                                &handle,
                                timer::tray_title(&guard.data.timer_state, now),
                                guard.data.timer_state.state,
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
                        window
                            .app_handle()
                            .state::<AppState>()
                            .panel_visible
                            .store(false, Ordering::SeqCst);
                    }
                }
            }
            if window.label() == "stats" {
                if let WindowEvent::Destroyed = event {
                    window
                        .app_handle()
                        .state::<AppState>()
                        .stats_open
                        .store(false, Ordering::SeqCst);
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
            set_task_planned,
            add_goal,
            update_goal,
            delete_goal,
            toggle_goal_today,
            open_stats,
            get_stats_bundle,
            clear_today_stats,
            set_history_task_done,
            export_backup,
            import_backup,
            get_notion_config,
            set_notion_config,
            log_notion_export,
            set_global_shortcut,
            get_autostart,
            set_autostart,
            finish_onboarding,
            close_celebration
        ])
        .run(context)
        .expect("error while running tauri application");
}
