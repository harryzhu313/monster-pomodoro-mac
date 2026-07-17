// 状态机,按行为对齐移植自旧 background/service-worker.js(以旧代码为行为规范)。
// 结构:纯函数操作 &mut StoreData,时间由调用方传入(可测);
// 副作用(音频/通知/庆祝)以 Effect 返回,由 lib.rs 的执行器消费——
// phase 02 只打日志,phase 04 接真实音频与通知。

use chrono::{Local, NaiveDate};
use serde::Serialize;
use serde_json::Value;

use crate::store::{
    BadgesState, StoreData, Task, TimerPhase, TimerState, DAILY_EXTEND_LIMIT, STREAK_GOAL,
};

/// 专注启动后的放弃宽限期:10 秒内 = 误点,不记烂番茄(sw.js FOCUS_START_GRACE_MS)
const FOCUS_START_GRACE_MS: i64 = 10 * 1000;
/// love monster 展示窗口(sw.js LOVE_CELEBRATION_REINJECT_MS)
const LOVE_CELEBRATION_MS: i64 = 15 * 1000;

// —— 时长配置:TOMATO_TEST=1 单一开关(AGENTS/ARCHITECTURE §8)——

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Durations {
    pub focus_ms: i64,
    pub break_ms: i64,
    pub last_minute_ms: i64,
    pub test_mode: bool,
}

pub fn durations() -> Durations {
    let test_mode = std::env::var("TOMATO_TEST").map(|v| v == "1").unwrap_or(false);
    if test_mode {
        Durations { focus_ms: 15_000, break_ms: 30_000, last_minute_ms: 5_000, test_mode }
    } else {
        Durations {
            focus_ms: 25 * 60 * 1000,
            break_ms: 5 * 60 * 1000,
            last_minute_ms: 60 * 1000,
            test_mode,
        }
    }
}

// —— 副作用(phase 02 仅日志;04 接音频/通知;07 接庆祝窗口)——

#[derive(Debug, Clone, PartialEq)]
pub enum Effect {
    Chime,
    WhiteNoiseStart,
    WhiteNoiseStop,
    /// body 中 {SUBTITLE} 占位符由 phase 04 用 MONSTER_SUBTITLES 池替换
    Notify { title: String, body: String },
    /// 7 天连击命中,phase 07 弹 love monster
    Celebration,
}

// —— 日期工具(本地时区,对齐旧 todayStr)——

pub fn today_str() -> String {
    Local::now().format("%Y-%m-%d").to_string()
}

fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

fn prior_day_str(date: &str) -> String {
    parse_date(date)
        .and_then(|d| d.pred_opt())
        .map(|d| d.format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| date.to_string())
}

fn days_between_str(from: &str, to: &str) -> i64 {
    match (parse_date(from), parse_date(to)) {
        (Some(a), Some(b)) => (b - a).num_days(),
        _ => 0,
    }
}

// —— 配额(惰性日切,sw.js:268-312)——

#[derive(Debug, Clone, Serialize)]
pub struct QuotaView {
    pub date: String,
    pub used: u32,
    pub limit: u32,
    pub remaining: u32,
}

pub fn get_quota(data: &StoreData, today: &str) -> QuotaView {
    let raw = &data.quota_state;
    let used = if raw.date == today {
        raw.used.min(DAILY_EXTEND_LIMIT)
    } else {
        0
    };
    QuotaView {
        date: today.to_string(),
        used,
        limit: DAILY_EXTEND_LIMIT,
        remaining: DAILY_EXTEND_LIMIT - used,
    }
}

/// 消费一次配额;成功则同时把今天标脏(连击清零依据,sw.js:294-304)
fn consume_quota(data: &mut StoreData, today: &str) -> bool {
    let q = get_quota(data, today);
    if q.remaining == 0 {
        return false;
    }
    data.quota_state.date = today.to_string();
    data.quota_state.used = q.used + 1;
    mark_extend_used_today(data, today);
    true
}

pub fn reset_quota(data: &mut StoreData, today: &str) {
    data.quota_state.date = today.to_string();
    data.quota_state.used = 0;
}

// —— 统计(sw.js:652-707)——

fn mutate_today_stats(data: &mut StoreData, today: &str, f: impl FnOnce(&mut crate::store::StatEntry)) {
    let entry = data.stats.entry(today.to_string()).or_default();
    f(entry);
    // 保留 366 天,供热力图展示整年
    if let Some(cutoff) = parse_date(today).and_then(|d| d.checked_sub_days(chrono::Days::new(366))) {
        let cutoff = cutoff.format("%Y-%m-%d").to_string();
        data.stats.retain(|k, _| k.as_str() >= cutoff.as_str());
    }
}

fn get_today_completed(data: &StoreData, today: &str) -> u32 {
    data.stats.get(today).map(|e| e.completed).unwrap_or(0)
}

// —— 任务跨天归档(sw.js:713-738)——

pub fn roll_over_tasks_if_needed(data: &mut StoreData, today: &str) {
    if data.tasks_today.date == today {
        return;
    }
    let old_date = std::mem::take(&mut data.tasks_today.date);
    let old_tasks = std::mem::take(&mut data.tasks_today.tasks);
    if !old_date.is_empty() && !old_tasks.is_empty() {
        let archived = data.tasks_archive.entry(old_date).or_default();
        // 按 id 去重合并(同一天多次归档时不产生重复)
        let seen: std::collections::HashSet<String> =
            archived.iter().map(|t| t.id.to_string()).collect();
        for task in old_tasks {
            if !seen.contains(&task.id.to_string()) {
                archived.push(task);
            }
        }
    }
    data.tasks_today.date = today.to_string();
}

fn current_task_mut(data: &mut StoreData) -> Option<&mut Task> {
    data.tasks_today.tasks.iter_mut().find(|t| t.is_current && !t.done)
}

// —— 连击 / 徽章(sw.js:779-900)——

fn compute_streak(raw: &BadgesState, today: &str, quota_used: u32, today_completed: u32) -> u32 {
    if quota_used > 0 {
        return 0;
    }
    let anchor = raw.anchor_date.clone().unwrap_or_else(|| today.to_string());
    let mut candidates = vec![prior_day_str(&anchor)];
    if let Some(d) = &raw.last_extend_date {
        candidates.push(d.clone());
    }
    if let Some(d) = raw.unlocked_dates.last() {
        candidates.push(d.clone());
    }
    candidates.sort();
    let base = candidates.last().cloned().unwrap_or_else(|| today.to_string());
    // 今天还没做番茄,按"截至昨天"计;做了今天才算上
    let endpoint = if today_completed > 0 { today.to_string() } else { prior_day_str(today) };
    days_between_str(&base, &endpoint).clamp(0, STREAK_GOAL as i64) as u32
}

/// 首次访问时用 stats 最早一天补 anchor(sw.js:797-812)
fn ensure_anchor(data: &mut StoreData, today: &str) {
    if data.badges_state.anchor_date.is_none() {
        let earliest = data.stats.keys().next().cloned();
        data.badges_state.anchor_date = Some(earliest.unwrap_or_else(|| today.to_string()));
    }
}

fn mark_extend_used_today(data: &mut StoreData, today: &str) {
    ensure_anchor(data, today);
    data.badges_state.last_extend_date = Some(today.to_string());
}

/// 进入休息时结算里程碑:今天第 1 个番茄 && streak≥7 && 配额干净 && 今天未解锁过 → 颁发
fn record_break_entry_milestone(data: &mut StoreData, today: &str) -> bool {
    ensure_anchor(data, today);
    let quota_used = get_quota(data, today).used;
    let today_completed = get_today_completed(data, today);
    if today_completed != 1 {
        return false;
    }
    let raw = &data.badges_state;
    let streak = compute_streak(raw, today, quota_used, today_completed);
    if streak >= STREAK_GOAL && quota_used == 0 && !raw.unlocked_dates.contains(&today.to_string()) {
        data.badges_state.badges += 1;
        data.badges_state.unlocked_dates.push(today.to_string());
        return true;
    }
    false
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgesView {
    pub badges: u32,
    pub current_streak: u32,
    pub goal: u32,
}

pub fn get_badges_view(data: &StoreData, today: &str) -> BadgesView {
    let quota_used = get_quota(data, today).used;
    let completed = get_today_completed(data, today);
    BadgesView {
        badges: data.badges_state.badges,
        current_streak: compute_streak(&data.badges_state, today, quota_used, completed),
        goal: STREAK_GOAL,
    }
}

// —— 长休息(sw.js:452-472)——

fn clamp_long_break_minutes(v: i64) -> u32 {
    let v = if v == 0 { 20 } else { v };
    v.clamp(15, 30) as u32
}

fn normalize_long_break_every(v: i64) -> u32 {
    let v = if v == 0 { 4 } else { v };
    v.clamp(2, 12) as u32
}

fn should_take_long_break(completed_today: u32, data: &StoreData) -> bool {
    if !data.settings.long_break_enabled {
        return false;
    }
    let every = normalize_long_break_every(data.settings.long_break_every as i64);
    completed_today > 0 && completed_today % every == 0
}

fn break_duration_ms(kind: &str, data: &StoreData, dur: &Durations) -> i64 {
    if kind != "long" || dur.test_mode {
        return dur.break_ms;
    }
    clamp_long_break_minutes(data.settings.long_break_minutes as i64) as i64 * 60 * 1000
}

// —— 状态转换 ——

pub fn start_focus(data: &mut StoreData, now: i64, dur: &Durations) {
    data.timer_state = TimerState {
        state: TimerPhase::Focusing,
        phase: Some("focus".into()),
        break_kind: None,
        end_time: Some(now + dur.focus_ms),
        paused_remaining: None,
        pre_pause_state: None,
        focus_started_at: Some(now),
        love_monster_until: None,
    };
}

/// 仅 FOCUSING 可暂停(break 不可暂停,sw.js:586-598)
pub fn pause(data: &mut StoreData, now: i64) {
    let s = &data.timer_state;
    if s.state != TimerPhase::Focusing {
        return;
    }
    let remaining = (s.end_time.unwrap_or(now) - now).max(0);
    data.timer_state.pre_pause_state = Some(s.state);
    data.timer_state.state = TimerPhase::Paused;
    data.timer_state.end_time = None;
    data.timer_state.paused_remaining = Some(remaining);
}

pub fn resume(data: &mut StoreData, now: i64) {
    let s = &data.timer_state;
    if s.state != TimerPhase::Paused {
        return;
    }
    let remaining = s.paused_remaining.unwrap_or(0);
    data.timer_state.state = s.pre_pause_state.unwrap_or(TimerPhase::Focusing);
    data.timer_state.end_time = Some(now + remaining);
    data.timer_state.paused_remaining = None;
    data.timer_state.pre_pause_state = None;
}

/// 红线 1:BREAKING / PAUSED-break 期间 reset 无效(无免费跳过);force 仅供内部收尾用
pub fn reset(data: &mut StoreData, force: bool) {
    let s = &data.timer_state;
    let in_break = s.state == TimerPhase::Breaking
        || (s.state == TimerPhase::Paused && s.phase.as_deref() == Some("break"));
    if !force && in_break {
        return;
    }
    data.timer_state = TimerState::default();
}

/// 只对进行中的专注生效;启动 10 秒宽限期内的放弃不污染统计(sw.js:622-636)
pub fn abandon(data: &mut StoreData, now: i64, today: &str) {
    let s = &data.timer_state;
    if s.phase.as_deref() != Some("focus") {
        return;
    }
    let in_grace = s
        .focus_started_at
        .map(|t| now - t < FOCUS_START_GRACE_MS)
        .unwrap_or(false);
    if !in_grace {
        mutate_today_stats(data, today, |e| e.rotten += 1);
        roll_over_tasks_if_needed(data, today);
        if let Some(task) = current_task_mut(data) {
            task.rotten += 1;
        }
    }
    data.timer_state = TimerState::default();
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OvertimeOutcome {
    pub ok: bool,
    pub reason: Option<String>,
}

/// 加时(旧 claimExtraTime,sw.js:316-341):仅 BREAKING/FOCUSING;扣配额;
/// → FOCUSING 专注 ms 毫秒,跑完走正常专注完成路径
pub fn claim_extra_time(data: &mut StoreData, ms: i64, now: i64, today: &str) -> OvertimeOutcome {
    if ms <= 0 {
        return OvertimeOutcome { ok: false, reason: Some("invalid-ms".into()) };
    }
    let state = data.timer_state.state;
    if state != TimerPhase::Breaking && state != TimerPhase::Focusing {
        return OvertimeOutcome { ok: false, reason: Some("wrong-state".into()) };
    }
    if !consume_quota(data, today) {
        return OvertimeOutcome { ok: false, reason: Some("quota-exhausted".into()) };
    }
    data.timer_state = TimerState {
        state: TimerPhase::Focusing,
        phase: Some("focus".into()),
        break_kind: None,
        end_time: Some(now + ms),
        paused_remaining: None,
        pre_pause_state: None,
        focus_started_at: Some(now),
        love_monster_until: None,
    };
    OvertimeOutcome { ok: true, reason: None }
}

fn start_break(data: &mut StoreData, now: i64, today: &str, dur: &Durations) -> Vec<Effect> {
    let completed_today = get_today_completed(data, today);
    let kind = if should_take_long_break(completed_today, data) { "long" } else { "short" };
    let awarded = record_break_entry_milestone(data, today);
    let duration = break_duration_ms(kind, data, dur);
    data.timer_state = TimerState {
        state: TimerPhase::Breaking,
        phase: Some("break".into()),
        break_kind: Some(kind.into()),
        end_time: Some(now + duration),
        paused_remaining: None,
        pre_pause_state: None,
        focus_started_at: None,
        love_monster_until: awarded.then(|| now + LOVE_CELEBRATION_MS),
    };
    let mut effects = vec![
        // 文案对齐 CONTENT.md"系统通知"节;{SUBTITLE} 由 phase 04 从 MONSTER_SUBTITLES 池填充
        Effect::Notify {
            title: "25 分钟到了,该停下来了".into(),
            body: "{SUBTITLE}".into(),
        },
        Effect::WhiteNoiseStart,
    ];
    if awarded {
        effects.push(Effect::Celebration);
    }
    effects
}

/// 到点结算。只在 FOCUSING/BREAKING 且 now >= endTime 时生效,由 tick 循环驱动。
/// 与旧版差异(有意):旧版 chime 后 sleep(1s) 再切状态是为了音频错峰,
/// 新版状态即时切换,音频错峰由 phase 04 的效果执行器处理——状态机不睡觉。
pub fn handle_phase_end_if_due(data: &mut StoreData, now: i64, today: &str, dur: &Durations) -> Vec<Effect> {
    let s = &data.timer_state;
    let due = matches!(s.state, TimerPhase::Focusing | TimerPhase::Breaking)
        && s.end_time.map(|t| now >= t).unwrap_or(false);
    if !due {
        return vec![];
    }

    if s.phase.as_deref() == Some("focus") {
        // 专注结束:记完成 → 当前任务 used+1 → 进休息(长/短)
        let mut effects = vec![Effect::Chime];
        mutate_today_stats(data, today, |e| e.completed += 1);
        roll_over_tasks_if_needed(data, today);
        if let Some(task) = current_task_mut(data) {
            task.used += 1;
        }
        effects.extend(start_break(data, now, today, dur));
        return effects;
    }

    // 休息结束:停白噪音 → chime → 自动/手动开始下一轮
    let mut effects = vec![Effect::WhiteNoiseStop, Effect::Chime];
    ensure_anchor(data, today);
    if data.settings.auto_start_next_focus {
        effects.push(Effect::Notify {
            title: "休息好了,回来吧".into(),
            body: "下一个番茄已经开始了".into(),
        });
        start_focus(data, now, dur);
    } else {
        effects.push(Effect::Notify {
            title: "休息好了,回来吧".into(),
            body: "准备好就开始下一个".into(),
        });
        reset(data, true);
    }
    effects
}

// —— settings 更新(patch 合并 + 数值 clamp)——

pub fn update_settings(data: &mut StoreData, patch: Value) {
    let Value::Object(patch) = patch else { return };
    let mut cur = match serde_json::to_value(&data.settings) {
        Ok(Value::Object(m)) => m,
        _ => return,
    };
    for (k, v) in patch {
        // JS 侧可能传浮点,数值字段先取整(对齐旧 Math.round(Number(v)))
        if matches!(k.as_str(), "longBreakEvery" | "longBreakMinutes") {
            if let Some(n) = v.as_f64() {
                cur.insert(k, Value::from(n.round() as i64));
                continue;
            }
        }
        cur.insert(k, v);
    }
    if let Ok(mut s) = serde_json::from_value::<crate::store::Settings>(Value::Object(cur)) {
        s.long_break_minutes = clamp_long_break_minutes(s.long_break_minutes as i64);
        s.long_break_every = normalize_long_break_every(s.long_break_every as i64);
        data.settings = s;
    }
}

// —— 前端快照(state-update 事件载荷,前端纯视图的唯一数据源)——

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub timer: TimerState,
    pub quota: QuotaView,
    pub settings: crate::store::Settings,
    pub durations: Durations,
    pub today: String,
    pub today_stats: crate::store::StatEntry,
    pub badges: BadgesView,
    pub tasks_today: crate::store::TasksToday,
    pub now: i64,
}

pub fn snapshot(data: &StoreData, now: i64, today: &str, dur: &Durations) -> Snapshot {
    Snapshot {
        timer: data.timer_state.clone(),
        quota: get_quota(data, today),
        settings: data.settings.clone(),
        durations: *dur,
        today: today.to_string(),
        today_stats: data.stats.get(today).copied().unwrap_or_default(),
        badges: get_badges_view(data, today),
        tasks_today: data.tasks_today.clone(),
        now,
    }
}

// —— 托盘标题(CONTENT.md 菜单栏节:专注 `24:59`,休息 `休息 4:32`,idle 无标题)——

pub fn tray_title(state: &TimerState, now: i64) -> Option<String> {
    let fmt_mmss = |ms: i64| {
        let total = (ms.max(0) + 999) / 1000;
        format!("{:02}:{:02}", total / 60, total % 60)
    };
    let fmt_break = |ms: i64| {
        let total = (ms.max(0) + 999) / 1000;
        format!("休息 {}:{:02}", total / 60, total % 60)
    };
    match state.state {
        TimerPhase::Focusing => Some(fmt_mmss(state.end_time? - now)),
        TimerPhase::Breaking => Some(fmt_break(state.end_time? - now)),
        // CONTENT.md 未定义暂停态;暂用 ⏸ 前缀,用户验收时确认
        TimerPhase::Paused => Some(format!("⏸ {}", fmt_mmss(state.paused_remaining?))),
        TimerPhase::Idle => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::StoreData;

    const DUR: Durations =
        Durations { focus_ms: 25 * 60 * 1000, break_ms: 5 * 60 * 1000, last_minute_ms: 60_000, test_mode: false };
    const T: &str = "2026-07-17";

    fn data_with_task() -> StoreData {
        let mut d = StoreData::default();
        d.tasks_today.date = T.into();
        d.tasks_today.tasks.push(Task {
            id: Value::from(1),
            title: "测试任务".into(),
            planned: 4,
            is_current: true,
            ..Task::default()
        });
        d
    }

    #[test]
    fn quota_lazy_day_reset() {
        let mut d = StoreData::default();
        d.quota_state.date = "2026-07-16".into();
        d.quota_state.used = 3;
        let q = get_quota(&d, T);
        assert_eq!((q.used, q.remaining), (0, 3)); // 昨天用光,今天满血
        assert!(consume_quota(&mut d, T));
        assert_eq!(d.badges_state.last_extend_date.as_deref(), Some(T)); // 连击标脏
    }

    #[test]
    fn quota_exhaustion() {
        let mut d = StoreData::default();
        for _ in 0..3 {
            assert!(consume_quota(&mut d, T));
        }
        assert!(!consume_quota(&mut d, T));
    }

    #[test]
    fn long_break_trigger() {
        let mut d = StoreData::default();
        assert!(!should_take_long_break(0, &d));
        assert!(should_take_long_break(4, &d));
        assert!(!should_take_long_break(5, &d));
        assert!(should_take_long_break(8, &d));
        d.settings.long_break_enabled = false;
        assert!(!should_take_long_break(4, &d));
    }

    #[test]
    fn clamps_match_old_behavior() {
        assert_eq!(clamp_long_break_minutes(0), 20); // Number(v)||20
        assert_eq!(clamp_long_break_minutes(40), 30);
        assert_eq!(clamp_long_break_minutes(10), 15);
        assert_eq!(normalize_long_break_every(0), 4);
        assert_eq!(normalize_long_break_every(1), 2);
        assert_eq!(normalize_long_break_every(99), 12);
    }

    #[test]
    fn streak_computation() {
        let mut raw = BadgesState::default();
        raw.anchor_date = Some("2026-07-10".into());
        // 干净 8 天,clamp 到 7
        assert_eq!(compute_streak(&raw, T, 0, 1), 7);
        // 今日用过配额 → 0
        assert_eq!(compute_streak(&raw, T, 1, 1), 0);
        // 今天还没完成番茄 → 只算到昨天
        assert_eq!(compute_streak(&raw, T, 0, 0), 7);
        // 7-14 用过延长 → 基点后移
        raw.last_extend_date = Some("2026-07-14".into());
        assert_eq!(compute_streak(&raw, T, 0, 1), 3);
    }

    #[test]
    fn milestone_awarded_only_on_first_tomato() {
        let mut d = StoreData::default();
        d.badges_state.anchor_date = Some("2026-07-01".into());
        d.stats.entry(T.into()).or_default().completed = 1;
        assert!(record_break_entry_milestone(&mut d, T));
        assert_eq!(d.badges_state.badges, 1);
        // 同日第二次不重复颁发(completed 已不是 1)
        d.stats.entry(T.into()).or_default().completed = 2;
        assert!(!record_break_entry_milestone(&mut d, T));
    }

    #[test]
    fn abandon_grace_period() {
        let now = 1_000_000_000;
        let mut d = data_with_task();
        start_focus(&mut d, now, &DUR);
        abandon(&mut d, now + 5_000, T); // 宽限期内
        assert_eq!(d.stats.get(T).map(|e| e.rotten).unwrap_or(0), 0);
        assert_eq!(d.timer_state.state, TimerPhase::Idle);

        start_focus(&mut d, now, &DUR);
        abandon(&mut d, now + 15_000, T); // 宽限期外
        assert_eq!(d.stats[T].rotten, 1);
        assert_eq!(d.tasks_today.tasks[0].rotten, 1);
    }

    #[test]
    fn no_free_exit_from_break() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        d.stats.entry(T.into()).or_default().completed = 1;
        start_break(&mut d, now, T, &DUR);
        assert_eq!(d.timer_state.state, TimerPhase::Breaking);
        reset(&mut d, false); // 红线 1:休息中 reset 无效
        assert_eq!(d.timer_state.state, TimerPhase::Breaking);
        abandon(&mut d, now, T); // abandon 对 break 无效
        assert_eq!(d.timer_state.state, TimerPhase::Breaking);
        pause(&mut d, now); // break 不可暂停
        assert_eq!(d.timer_state.state, TimerPhase::Breaking);
    }

    #[test]
    fn overtime_flow() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        d.stats.entry(T.into()).or_default().completed = 1;
        start_break(&mut d, now, T, &DUR);
        let r = claim_extra_time(&mut d, 5 * 60 * 1000, now, T);
        assert!(r.ok);
        assert_eq!(d.timer_state.state, TimerPhase::Focusing);
        assert_eq!(get_quota(&d, T).used, 1);
        // 配额耗尽后拒绝
        d.quota_state.used = 3;
        let r = claim_extra_time(&mut d, 5000, now, T);
        assert_eq!(r.reason.as_deref(), Some("quota-exhausted"));
    }

    #[test]
    fn focus_end_records_and_starts_break() {
        let mut d = data_with_task();
        let now = 1_000_000_000;
        start_focus(&mut d, now, &DUR);
        let effects = handle_phase_end_if_due(&mut d, now + DUR.focus_ms, T, &DUR);
        assert_eq!(d.stats[T].completed, 1);
        assert_eq!(d.tasks_today.tasks[0].used, 1);
        assert_eq!(d.timer_state.state, TimerPhase::Breaking);
        assert_eq!(d.timer_state.break_kind.as_deref(), Some("short"));
        assert!(effects.contains(&Effect::Chime));
        assert!(effects.contains(&Effect::WhiteNoiseStart));
        // 未到点不结算
        let effects = handle_phase_end_if_due(&mut d, now, T, &DUR);
        assert!(effects.is_empty());
    }

    #[test]
    fn fourth_tomato_gets_long_break() {
        let mut d = StoreData::default();
        d.stats.entry(T.into()).or_default().completed = 3; // 本次结束后 = 4
        let now = 1_000_000_000;
        start_focus(&mut d, now, &DUR);
        handle_phase_end_if_due(&mut d, now + DUR.focus_ms, T, &DUR);
        assert_eq!(d.timer_state.break_kind.as_deref(), Some("long"));
        // 长休息 20 分钟(默认)
        assert_eq!(d.timer_state.end_time.unwrap() - (now + DUR.focus_ms), 20 * 60 * 1000);
    }

    #[test]
    fn break_end_autostarts_or_resets() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        d.stats.entry(T.into()).or_default().completed = 1;
        start_break(&mut d, now, T, &DUR);
        let end = d.timer_state.end_time.unwrap();
        let effects = handle_phase_end_if_due(&mut d, end, T, &DUR);
        assert_eq!(d.timer_state.state, TimerPhase::Focusing); // 默认自动开始
        assert!(effects.contains(&Effect::WhiteNoiseStop));

        d.settings.auto_start_next_focus = false;
        start_break(&mut d, now, T, &DUR);
        let end = d.timer_state.end_time.unwrap();
        handle_phase_end_if_due(&mut d, end, T, &DUR);
        assert_eq!(d.timer_state.state, TimerPhase::Idle);
    }

    #[test]
    fn pause_resume_preserves_remaining() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        start_focus(&mut d, now, &DUR);
        pause(&mut d, now + 60_000);
        assert_eq!(d.timer_state.paused_remaining, Some(DUR.focus_ms - 60_000));
        resume(&mut d, now + 300_000);
        assert_eq!(d.timer_state.end_time, Some(now + 300_000 + DUR.focus_ms - 60_000));
    }

    #[test]
    fn task_rollover_archives_by_id() {
        let mut d = data_with_task();
        d.tasks_today.date = "2026-07-16".into();
        roll_over_tasks_if_needed(&mut d, T);
        assert_eq!(d.tasks_today.date, T);
        assert!(d.tasks_today.tasks.is_empty());
        assert_eq!(d.tasks_archive["2026-07-16"].len(), 1);
    }

    #[test]
    fn tray_title_formats() {
        let mut s = TimerState::default();
        assert_eq!(tray_title(&s, 0), None); // idle 无标题
        s.state = TimerPhase::Focusing;
        s.end_time = Some(24 * 60 * 1000 + 59_000);
        assert_eq!(tray_title(&s, 0).unwrap(), "24:59");
        s.state = TimerPhase::Breaking;
        s.end_time = Some(4 * 60 * 1000 + 32_000);
        assert_eq!(tray_title(&s, 0).unwrap(), "休息 4:32");
    }
}
