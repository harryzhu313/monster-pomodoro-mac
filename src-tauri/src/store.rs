// JSON 持久化。schema 顶层键沿用旧插件命名(ARCHITECTURE §4):
// timerState / quotaState / settings / stats / tasksToday / tasksArchive /
// badgesState / lastCategory / notionExportLog
// 原子写:先写临时文件再 rename;文件权限 600(store.json 未来存 Notion token)。

use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;

use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{Map, Value};

pub const DAILY_EXTEND_LIMIT: u32 = 3;
pub const STREAK_GOAL: u32 = 7;

// —— timerState ——

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimerPhase {
    #[serde(rename = "IDLE")]
    Idle,
    #[serde(rename = "FOCUSING")]
    Focusing,
    #[serde(rename = "BREAKING")]
    Breaking,
    #[serde(rename = "PAUSED")]
    Paused,
}

impl Default for TimerPhase {
    fn default() -> Self {
        TimerPhase::Idle
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TimerState {
    pub state: TimerPhase,
    /// null | "focus" | "break"
    pub phase: Option<String>,
    /// null | "short" | "long"
    pub break_kind: Option<String>,
    /// 当前阶段结束的 epoch 毫秒(endTime 制,零漂移的根基)
    pub end_time: Option<i64>,
    /// 暂停时剩余毫秒
    pub paused_remaining: Option<i64>,
    /// 暂停前的状态,恢复用
    pub pre_pause_state: Option<TimerPhase>,
    /// 本次专注启动时间;启动 10 秒内放弃不记烂番茄
    pub focus_started_at: Option<i64>,
    /// 7 天奖励命中时,休息开始后一小段时间内展示 love monster
    pub love_monster_until: Option<i64>,
    /// 本次休息的副标题(进入休息时从 MONSTER_SUBTITLES 池随机挑,面板与通知共用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub break_subtitle: Option<String>,
}

// —— quotaState(惰性日切:date 不是今天就视为全新一天)——

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct QuotaState {
    pub date: String,
    pub used: u32,
}

// —— settings(默认值对齐旧 DEFAULT_SETTINGS;lockscreenBg 为浏览器特有,弃)——

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub auto_start_next_focus: bool,
    pub white_noise_enabled: bool,
    pub chime_enabled: bool,
    pub long_break_enabled: bool,
    pub long_break_every: u32,
    pub long_break_minutes: u32,
    /// 最后一分钟提示(新设置项,旧版挂在 chimeEnabled 下,PRD §5 拆为独立开关)
    pub last_minute_enabled: bool,
    /// 全局快捷键(Tauri accelerator 格式;空串 = 关闭)
    pub global_shortcut: String,
    pub theme: String,
    /// 未识别字段原样保留,前向兼容(如未来版本的新设置)
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            auto_start_next_focus: true,
            white_noise_enabled: true,
            chime_enabled: true,
            long_break_enabled: true,
            long_break_every: 4,
            long_break_minutes: 20,
            last_minute_enabled: true,
            global_shortcut: "Ctrl+Alt+P".into(), // ⌃⌥P(CONTENT.md 首启引导文案)
            // 与旧版默认(default,隐藏小怪兽)不同:用户 2026-07-17 拍板默认展示小怪兽
            theme: "monster".into(),
            extra: Map::new(),
        }
    }
}

// —— stats:date → {completed, rotten}。旧数据可能是裸数字(只有 completed),读时归一化 ——

#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct StatEntry {
    pub completed: u32,
    pub rotten: u32,
}

impl<'de> Deserialize<'de> for StatEntry {
    fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        fn as_u32(v: Option<&Value>) -> u32 {
            v.and_then(Value::as_f64).map(|n| n.max(0.0) as u32).unwrap_or(0)
        }
        let v = Value::deserialize(d)?;
        Ok(match &v {
            Value::Number(n) => StatEntry {
                completed: n.as_f64().map(|x| x.max(0.0) as u32).unwrap_or(0),
                rotten: 0,
            },
            Value::Object(m) => StatEntry {
                completed: as_u32(m.get("completed")),
                rotten: as_u32(m.get("rotten")),
            },
            _ => StatEntry::default(),
        })
    }
}

// —— 任务:字段对齐旧 popup.js 的结构,未识别字段透传 ——

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Task {
    /// 旧版用 Date.now() 数字做 id,不假设类型
    pub id: Value,
    pub title: String,
    pub planned: u32,
    pub used: u32,
    pub rotten: u32,
    pub done: bool,
    pub is_current: bool,
    pub category: Option<String>,
    /// 手动勾选/取消完成的覆盖标记(旧 popup.js toggleTaskDone 行为,历史明细展示用)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub done_override: Option<bool>,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct TasksToday {
    pub date: String,
    pub tasks: Vec<Task>,
}

// —— badgesState(连击 / 徽章)——

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct BadgesState {
    pub badges: u32,
    pub unlocked_dates: Vec<String>,
    pub last_extend_date: Option<String>,
    pub anchor_date: Option<String>,
}

// —— Notion 配置(token 只存本地 store.json,权限 600;不进备份文件)——

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct NotionConfig {
    pub token: String,
    pub task_db_id: String,
    pub day_db_id: String,
}

// —— 顶层 ——

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StoreData {
    pub timer_state: TimerState,
    pub quota_state: QuotaState,
    pub settings: Settings,
    pub stats: BTreeMap<String, StatEntry>,
    pub tasks_today: TasksToday,
    pub tasks_archive: BTreeMap<String, Vec<Task>>,
    pub badges_state: BadgesState,
    pub last_category: Option<String>,
    pub notion_export_log: Map<String, Value>,
    pub notion_config: NotionConfig,
    /// 首启引导已完成(既有安装在 Store::load 时自动补 true)
    pub onboarding_done: bool,
    #[serde(flatten)]
    pub extra: Map<String, Value>,
}

pub struct Store {
    path: PathBuf,
    pub data: StoreData,
}

impl Store {
    /// 读失败不覆盖原文件:损坏的 store.json 侧移为 .corrupt 保留现场,再用默认值起步
    pub fn load(path: PathBuf) -> Store {
        let data = match fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<StoreData>(&text) {
                Ok(mut data) => {
                    // 引导标记是后加字段:既有安装(文件已存在)视为不需要引导
                    data.onboarding_done = true;
                    data
                }
                Err(e) => {
                    eprintln!("store: 解析 {} 失败({e}),侧移为 .corrupt", path.display());
                    let _ = fs::rename(&path, path.with_extension("json.corrupt"));
                    StoreData::default()
                }
            },
            Err(_) => StoreData::default(),
        };
        Store { path, data }
    }

    /// 原子写:临时文件(权限 600)+ rename
    pub fn save(&self) -> io::Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        let text = serde_json::to_string_pretty(&self.data)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        fs::write(&tmp, text)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))?;
        }
        fs::rename(&tmp, &self.path)
    }
}

// —— 备份导入 / 导出(ARCHITECTURE §4)——
// 旧插件 v1 格式:{schemaVersion:1, exportedAt, appVersion, data:{stats, tasksToday,
// tasksArchive, badgesState, settings, lastTaskCategory, notionExportLog}}
// 新版导出 v2:结构相同,键名 lastCategory;导入兼容 v1/v2。
// timerState / quotaState / notionConfig 不在备份范围(旧行为一致;token 不外泄)。

pub const BACKUP_SCHEMA_VERSION: u64 = 2;

/// settings 里浏览器特有项,导入时丢弃(ARCHITECTURE §4)
const BROWSER_ONLY_SETTINGS: [&str; 1] = ["lockscreenBg"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportOutcome {
    pub ok: bool,
    pub error: Option<String>,
    /// 导入后 stats 覆盖的天数(成功文案用)
    pub days: u32,
}

pub fn export_backup_json(data: &StoreData, app_version: &str) -> String {
    let payload = serde_json::json!({
        "schemaVersion": BACKUP_SCHEMA_VERSION,
        "exportedAt": chrono::Local::now().to_rfc3339(),
        "appVersion": app_version,
        "data": {
            "stats": data.stats,
            "tasksToday": data.tasks_today,
            "tasksArchive": data.tasks_archive,
            "badgesState": data.badges_state,
            "settings": data.settings,
            "lastCategory": data.last_category,
            "notionExportLog": data.notion_export_log,
        }
    });
    serde_json::to_string_pretty(&payload).unwrap_or_default()
}

/// 导入:先在副本上完成全部校验与解析,全部成功才替换——任一步失败,现有数据零改动。
/// 语义对齐旧 importBackupFile:备份里有的键整体覆盖,没有的键回默认值。
pub fn import_backup(data: &mut StoreData, text: &str) -> Result<u32, String> {
    use serde::de::DeserializeOwned;

    let payload: Value =
        serde_json::from_str(text).map_err(|_| "备份文件不是有效的 JSON".to_string())?;
    let obj = payload.as_object().ok_or("备份文件不是有效的 JSON 对象")?;
    let version = obj.get("schemaVersion").and_then(Value::as_u64).unwrap_or(0);
    if version != 1 && version != BACKUP_SCHEMA_VERSION {
        return Err(format!("不支持的备份版本:{version}"));
    }
    let d = obj
        .get("data")
        .and_then(Value::as_object)
        .ok_or("备份文件缺少 data 字段")?;

    fn parse_key<T: DeserializeOwned>(
        d: &Map<String, Value>,
        key: &str,
        what: &str,
    ) -> Result<Option<T>, String> {
        match d.get(key) {
            None | Some(Value::Null) => Ok(None),
            Some(v) => serde_json::from_value(v.clone())
                .map(Some)
                .map_err(|e| format!("{what} 字段格式无效({e})")),
        }
    }

    let mut next = data.clone();
    let mut applied = false;
    let mut apply = |present: bool| {
        applied |= present;
    };

    let stats = parse_key::<BTreeMap<String, StatEntry>>(d, "stats", "stats")?;
    apply(stats.is_some());
    next.stats = stats.unwrap_or_default();

    let tasks_today = parse_key::<TasksToday>(d, "tasksToday", "tasksToday")?;
    apply(tasks_today.is_some());
    next.tasks_today = tasks_today.unwrap_or_default();

    let tasks_archive =
        parse_key::<BTreeMap<String, Vec<Task>>>(d, "tasksArchive", "tasksArchive")?;
    apply(tasks_archive.is_some());
    next.tasks_archive = tasks_archive.unwrap_or_default();

    let badges = parse_key::<BadgesState>(d, "badgesState", "badgesState")?;
    apply(badges.is_some());
    next.badges_state = badges.unwrap_or_default();

    // settings:先剔除浏览器特有键再解析
    match d.get("settings") {
        None | Some(Value::Null) => next.settings = Settings::default(),
        Some(v) => {
            let mut v = v.clone();
            if let Value::Object(ref mut m) = v {
                for key in BROWSER_ONLY_SETTINGS {
                    m.remove(key);
                }
            }
            next.settings = serde_json::from_value(v)
                .map_err(|e| format!("settings 字段格式无效({e})"))?;
            apply(true);
        }
    }

    // v2 用 lastCategory,v1 用 lastTaskCategory——两个键名都认
    let last_category = d
        .get("lastCategory")
        .or_else(|| d.get("lastTaskCategory"))
        .and_then(Value::as_str)
        .map(String::from);
    apply(last_category.is_some());
    next.last_category = last_category;

    let export_log = parse_key::<Map<String, Value>>(d, "notionExportLog", "notionExportLog")?;
    apply(export_log.is_some());
    next.notion_export_log = export_log.unwrap_or_default();

    if !applied {
        return Err("备份文件里没有可恢复的数据".to_string());
    }

    let days = next.stats.len() as u32;
    *data = next;
    Ok(days)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stat_entry_normalizes_legacy_number() {
        let stats: BTreeMap<String, StatEntry> =
            serde_json::from_str(r#"{"2026-07-01": 5, "2026-07-02": {"completed": 3, "rotten": 1}}"#)
                .unwrap();
        assert_eq!(stats["2026-07-01"].completed, 5);
        assert_eq!(stats["2026-07-01"].rotten, 0);
        assert_eq!(stats["2026-07-02"].completed, 3);
        assert_eq!(stats["2026-07-02"].rotten, 1);
    }

    #[test]
    fn store_data_uses_legacy_top_level_keys() {
        let v = serde_json::to_value(StoreData::default()).unwrap();
        for key in [
            "timerState", "quotaState", "settings", "stats", "tasksToday",
            "tasksArchive", "badgesState", "lastCategory", "notionExportLog",
        ] {
            assert!(v.get(key).is_some(), "missing key {key}");
        }
    }

    fn v1_backup_sample() -> String {
        r#"{
          "schemaVersion": 1,
          "exportedAt": "2026-07-01T10:00:00.000Z",
          "appVersion": "1.9",
          "data": {
            "stats": { "2026-06-30": 5, "2026-07-01": { "completed": 3, "rotten": 1 } },
            "tasksToday": { "date": "2026-07-01", "tasks": [
              { "id": 1719800000000, "title": "旧任务", "planned": 4, "used": 2,
                "done": false, "isCurrent": true, "category": "学习" } ] },
            "tasksArchive": { "2026-06-30": [
              { "id": "t_abc", "title": "归档任务", "planned": 1, "used": 1, "done": true } ] },
            "badgesState": { "badges": 2, "unlockedDates": ["2026-05-01", "2026-06-15"],
              "lastExtendDate": "2026-06-20", "anchorDate": "2026-04-01" },
            "settings": { "lockscreenBg": "dawn", "chimeEnabled": false, "longBreakEvery": 6,
              "theme": "monster" },
            "lastTaskCategory": "学习",
            "notionExportLog": { "2026-06-30": { "ok": true, "created": 1 } }
          }
        }"#
        .to_string()
    }

    #[test]
    fn import_v1_backup_maps_all_keys() {
        let mut d = StoreData::default();
        d.timer_state.state = TimerPhase::Focusing; // 备份范围外,应原样保留
        d.quota_state.used = 2;
        let days = import_backup(&mut d, &v1_backup_sample()).unwrap();
        assert_eq!(days, 2);
        assert_eq!(d.stats["2026-06-30"].completed, 5); // 旧数字格式归一化
        assert_eq!(d.stats["2026-07-01"].rotten, 1);
        assert_eq!(d.tasks_today.tasks[0].title, "旧任务");
        assert_eq!(d.tasks_archive["2026-06-30"].len(), 1);
        assert_eq!(d.badges_state.badges, 2);
        assert!(!d.settings.chime_enabled);
        assert_eq!(d.settings.long_break_every, 6);
        // 浏览器特有项丢弃
        assert!(!d.settings.extra.contains_key("lockscreenBg"));
        // 旧键名 lastTaskCategory → lastCategory
        assert_eq!(d.last_category.as_deref(), Some("学习"));
        assert_eq!(d.notion_export_log["2026-06-30"]["created"], 1);
        // 备份范围外零改动
        assert_eq!(d.timer_state.state, TimerPhase::Focusing);
        assert_eq!(d.quota_state.used, 2);
    }

    #[test]
    fn import_rejects_bad_files_without_touching_data() {
        let mut d = StoreData::default();
        d.stats.entry("2026-07-17".into()).or_default().completed = 9;
        let before = serde_json::to_string(&d).unwrap();

        for (name, text) in [
            ("损坏 JSON", "{not json"),
            ("版本不对", r#"{"schemaVersion": 99, "data": {"stats": {}}}"#),
            ("缺 data", r#"{"schemaVersion": 1}"#),
            ("空 data", r#"{"schemaVersion": 1, "data": {}}"#),
            ("stats 类型错", r#"{"schemaVersion": 1, "data": {"stats": "oops"}}"#),
        ] {
            let r = import_backup(&mut d, text);
            assert!(r.is_err(), "{name} 应该被拒绝");
            assert_eq!(serde_json::to_string(&d).unwrap(), before, "{name} 不得改动数据");
        }
    }

    #[test]
    fn backup_round_trip_v2() {
        let mut d = StoreData::default();
        import_backup(&mut d, &v1_backup_sample()).unwrap();
        let exported = export_backup_json(&d, "0.1.0");
        let mut d2 = StoreData::default();
        let days = import_backup(&mut d2, &exported).unwrap();
        assert_eq!(days, 2);
        assert_eq!(
            serde_json::to_value(&d.stats).unwrap(),
            serde_json::to_value(&d2.stats).unwrap()
        );
        assert_eq!(d.last_category, d2.last_category);
        assert_eq!(d.badges_state.badges, d2.badges_state.badges);
        // 导出不包含 notionConfig(token 不外泄)
        assert!(!exported.contains("notionConfig"));
    }

    #[test]
    fn settings_keeps_unknown_fields() {
        let s: Settings =
            serde_json::from_str(r#"{"chimeEnabled": false, "futureThing": 42}"#).unwrap();
        assert!(!s.chime_enabled);
        assert!(s.auto_start_next_focus); // 默认值补齐
        assert_eq!(s.extra["futureThing"], 42);
        let back = serde_json::to_value(&s).unwrap();
        assert_eq!(back["futureThing"], 42);
    }
}
