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
                Ok(data) => data,
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
