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
    /// 最后一分钟的轻提示音(G5→C6,比 chime 短)。纯声音,无系统通知
    SoftNudge,
    WhiteNoiseStart,
    WhiteNoiseStop,
    /// 休息开始:面板自动弹出并钉住——不抢键盘焦点,点任何地方都不收起
    /// (2026-07-17 用户决策:替代系统通知,通知三处全删)
    PanelPin,
    /// 休息结束/加时成功:解除钉住并收起面板
    PanelUnpin,
    /// 7 天连击命中,phase 07 弹 love monster
    Celebration,
}

/// 品牌语气文案池(平移旧 content/lockscreen.js:435 MONSTER_SUBTITLES,一字不改)
pub const MONSTER_SUBTITLES: [&str; 8] = [
    "坐了这么久,你的眼睛和脖子都不高兴了。",
    "屏幕看够了,眼睛在小声求你眨一眨。",
    "你的腰在偷偷抗议,先站起来走两步吧。",
    "肩膀已经硬得像块砖,该活动一下了。",
    "盯这么久,眼睛快要罢工啦。",
    "脖子想回到它本来的角度,让它歇会儿。",
    "站起来,活动一下吧。",
    "你的手腕也需要一点自由时间。",
];

fn pick_subtitle(now: i64) -> String {
    MONSTER_SUBTITLES[(now as usize) % MONSTER_SUBTITLES.len()].to_string()
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

/// 返回是否真的发生了归档(日期变了)——调用方据此决定要不要落盘 + 广播
pub fn roll_over_tasks_if_needed(data: &mut StoreData, today: &str) -> bool {
    if data.tasks_today.date == today {
        return false;
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
    true
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
        break_subtitle: None,
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
        break_subtitle: None,
    };
    OvertimeOutcome { ok: true, reason: None }
}

fn start_break(data: &mut StoreData, now: i64, today: &str, dur: &Durations) -> Vec<Effect> {
    let completed_today = get_today_completed(data, today);
    let kind = if should_take_long_break(completed_today, data) { "long" } else { "short" };
    let awarded = record_break_entry_milestone(data, today);
    let duration = break_duration_ms(kind, data, dur);
    // 面板副标题与系统通知正文共用同一句(CONTENT.md:正文从 MONSTER_SUBTITLES 随机挑)
    let subtitle = pick_subtitle(now);
    data.timer_state = TimerState {
        state: TimerPhase::Breaking,
        phase: Some("break".into()),
        break_kind: Some(kind.into()),
        end_time: Some(now + duration),
        paused_remaining: None,
        pre_pause_state: None,
        focus_started_at: None,
        love_monster_until: awarded.then(|| now + LOVE_CELEBRATION_MS),
        // 副标题经 timerState.breakSubtitle 到达面板(系统通知已按用户决策移除)
        break_subtitle: Some(subtitle),
    };
    let mut effects = vec![Effect::PanelPin, Effect::WhiteNoiseStart];
    if awarded {
        effects.push(Effect::Celebration);
    }
    effects
}

/// 最后一分钟提示:FOCUSING 剩余进入最后一分钟窗口时触发一次(按 endTime 去重,
/// 等价旧 LAST_MINUTE_STATE_KEY 机制)。fired 由调用方持有(内存态,不持久化)。
pub fn check_last_minute(
    data: &StoreData,
    now: i64,
    dur: &Durations,
    fired: &mut Option<i64>,
) -> Vec<Effect> {
    let s = &data.timer_state;
    if s.state != TimerPhase::Focusing {
        return vec![];
    }
    let Some(end) = s.end_time else { return vec![] };
    if *fired == Some(end) {
        return vec![];
    }
    let remaining = end - now;
    if remaining <= 0 || remaining > dur.last_minute_ms {
        return vec![];
    }
    *fired = Some(end); // 关掉开关时也记为已处理,避免中途打开立刻响
    if !data.settings.last_minute_enabled {
        return vec![];
    }
    vec![Effect::SoftNudge]
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

    // 休息结束:停白噪音 → chime → 收起面板 → 自动/手动开始下一轮
    // (自动/手动模式都收起面板,用户 2026-07-17 拍板)
    let effects = vec![Effect::WhiteNoiseStop, Effect::Chime, Effect::PanelUnpin];
    ensure_anchor(data, today);
    if data.settings.auto_start_next_focus {
        start_focus(data, now, dur);
    } else {
        reset(data, true);
    }
    effects
}

// —— 任务管理(平移旧 popup.js:400-444;分类常量是 Notion select 依赖,不可改——红线 2)——

const CATEGORIES: [&str; 4] = ["工作", "学习", "生活", "兴趣爱好"];

fn normalize_category(c: Option<String>) -> String {
    match c {
        Some(c) if CATEGORIES.contains(&c.as_str()) => c,
        _ => "工作".to_string(),
    }
}

/// 旧 popup 用 String(id) 做比较,这里对齐:类型不同但字符串形式相同也算同一任务
fn task_id_eq(a: &Value, b: &Value) -> bool {
    a == b || a.to_string() == b.to_string()
}

pub fn add_task(
    data: &mut StoreData,
    today: &str,
    now: i64,
    title: &str,
    category: Option<String>,
    planned: f64,
) {
    let title = title.trim();
    if title.is_empty() {
        return;
    }
    roll_over_tasks_if_needed(data, today);
    let planned = if planned.is_finite() { planned.floor() as i64 } else { 1 }.clamp(1, 20) as u32;
    let category = normalize_category(category);
    // 没有活跃的当前任务时,新任务自动成为当前(旧 addTask 的 isCurrent 规则)
    let is_current = data.tasks_today.tasks.iter().all(|t| !t.is_current || t.done);
    let id = format!("t_{:x}_{:04x}", now, (now as u64).wrapping_mul(2_654_435_761) & 0xffff);
    data.tasks_today.tasks.push(Task {
        id: Value::String(id),
        title: title.to_string(),
        planned,
        is_current,
        category: Some(category.clone()),
        ..Task::default()
    });
    data.last_category = Some(category);
}

/// 改今天这条任务的计划番茄数(v2)。夹紧到 [max(1, used), 99]:
/// 已经做了 3 个就不能改到 2,否则历史记录对不上。
pub fn set_task_planned(data: &mut StoreData, today: &str, id: &Value, planned: f64) {
    roll_over_tasks_if_needed(data, today);
    for t in &mut data.tasks_today.tasks {
        if task_id_eq(&t.id, id) {
            let n = if planned.is_finite() { planned.floor() as i64 } else { 1 };
            t.planned = n.clamp(t.used.max(1) as i64, 99) as u32;
            return;
        }
    }
}

// —— 月度目标(v2)——
// 目标本身长期有效,不按月分桶;"本月进度"是算出来的,不落库。

fn month_prefix(today: &str) -> &str {
    // "2026-08-24" → "2026-08";日期串格式由 today_str 保证
    if today.len() >= 7 { &today[..7] } else { today }
}

/// 本月已投入番茄数 = 今日任务 + 归档里本月范围内、goalId 匹配的任务的 used 之和
fn goal_month_done(data: &StoreData, today: &str, goal_id: &str) -> u32 {
    let m = month_prefix(today);
    let matches = |t: &Task| t.goal_id.as_deref() == Some(goal_id);
    let mut sum = 0u32;
    if data.tasks_today.date.starts_with(m) {
        sum += data.tasks_today.tasks.iter().filter(|t| matches(t)).map(|t| t.used).sum::<u32>();
    }
    for (date, tasks) in &data.tasks_archive {
        if date.starts_with(m) {
            sum += tasks.iter().filter(|t| matches(t)).map(|t| t.used).sum::<u32>();
        }
    }
    sum
}

/// 今天由该目标产生的任务(至多一条)
fn goal_task_today<'a>(data: &'a StoreData, goal_id: &str) -> Option<&'a Task> {
    data.tasks_today.tasks.iter().find(|t| t.goal_id.as_deref() == Some(goal_id))
}

pub fn add_goal(data: &mut StoreData, now: i64, title: &str, category: Option<String>, month_target: f64) {
    let title = title.trim();
    if title.is_empty() {
        return;
    }
    let month_target =
        if month_target.is_finite() { month_target.floor() as i64 } else { 1 }.clamp(1, 999) as u32;
    let id = format!("g_{:x}_{:04x}", now, (now as u64).wrapping_mul(2_654_435_761) & 0xffff);
    data.monthly_goals.push(crate::store::MonthlyGoal {
        id,
        title: title.to_string(),
        category: Some(normalize_category(category)),
        month_target,
        ..Default::default()
    });
}

/// 只更新传入的字段(None = 不动)
pub fn update_goal(
    data: &mut StoreData,
    id: &str,
    title: Option<String>,
    category: Option<String>,
    month_target: Option<f64>,
) {
    let Some(g) = data.monthly_goals.iter_mut().find(|g| g.id == id) else { return };
    if let Some(t) = title {
        let t = t.trim().to_string();
        if !t.is_empty() {
            g.title = t;
        }
    }
    if let Some(c) = category {
        g.category = Some(normalize_category(Some(c)));
    }
    if let Some(n) = month_target {
        let n = if n.is_finite() { n.floor() as i64 } else { 1 };
        g.month_target = n.clamp(1, 999) as u32;
    }
    // 目标改名后,今天那条由它产生的任务跟着改,避免面板上两处名字对不上
    let (title, category) = (g.title.clone(), g.category.clone());
    let gid = id.to_string();
    for t in &mut data.tasks_today.tasks {
        if t.goal_id.as_deref() == Some(gid.as_str()) {
            t.title = title.clone();
            t.category = category.clone();
        }
    }
}

/// 删除目标。已经产生的任务(今日与历史)一律保留——删目标不该抹掉已有记录,
/// 它们的 goalId 变成悬空引用,只影响本月进度不再计入,不影响任务本身。
pub fn delete_goal(data: &mut StoreData, id: &str) {
    data.monthly_goals.retain(|g| g.id != id);
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalToggleOutcome {
    pub ok: bool,
    /// null | "locked"(今天已投入番茄,不可取消) | "not-found"
    pub reason: Option<String>,
    pub checked: bool,
}

/// 勾选 = 按目标预设建今日任务(番茄数默认 1);取消 = 删掉那条任务。
/// 今天已投入番茄的目标拒绝取消,否则会连带抹掉已有的番茄记录。
pub fn toggle_goal_today(data: &mut StoreData, today: &str, now: i64, id: &str) -> GoalToggleOutcome {
    roll_over_tasks_if_needed(data, today);
    let Some(goal) = data.monthly_goals.iter().find(|g| g.id == id).cloned() else {
        return GoalToggleOutcome { ok: false, reason: Some("not-found".into()), checked: false };
    };

    match goal_task_today(data, id) {
        Some(t) => {
            if t.used > 0 {
                return GoalToggleOutcome {
                    ok: false,
                    reason: Some("locked".into()),
                    checked: true,
                };
            }
            let tid = t.id.clone();
            delete_task(data, today, &tid);
            GoalToggleOutcome { ok: true, reason: None, checked: false }
        }
        None => {
            let is_current = data.tasks_today.tasks.iter().all(|t| !t.is_current || t.done);
            let tid = format!("t_{:x}_{:04x}", now, (now as u64).wrapping_mul(2_654_435_761) & 0xffff);
            data.tasks_today.tasks.push(Task {
                id: Value::String(tid),
                title: goal.title.clone(),
                planned: 1, // 今天做几个在面板上敲,预设只决定名称与分类
                is_current,
                category: goal.category.clone(),
                goal_id: Some(goal.id.clone()),
                ..Task::default()
            });
            GoalToggleOutcome { ok: true, reason: None, checked: true }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GoalView {
    pub id: String,
    pub title: String,
    pub category: String,
    pub month_target: u32,
    pub month_done: u32,
    pub checked_today: bool,
    /// 今天已投入番茄 → chip 不可取消勾选
    pub locked: bool,
}

pub fn goal_views(data: &StoreData, today: &str) -> Vec<GoalView> {
    data.monthly_goals
        .iter()
        .map(|g| {
            let t = goal_task_today(data, &g.id);
            GoalView {
                id: g.id.clone(),
                title: g.title.clone(),
                category: normalize_category(g.category.clone()),
                month_target: g.month_target,
                month_done: goal_month_done(data, today, &g.id),
                checked_today: t.is_some(),
                locked: t.map(|t| t.used > 0).unwrap_or(false),
            }
        })
        .collect()
}

pub fn set_task_done(data: &mut StoreData, today: &str, id: &Value, done: bool) {
    roll_over_tasks_if_needed(data, today);
    for t in &mut data.tasks_today.tasks {
        if task_id_eq(&t.id, id) {
            t.done = done;
            t.done_override = Some(done); // 历史明细的圆圈状态与勾选一致(旧 toggleTaskDone)
            if done {
                t.is_current = false;
            }
        }
    }
}

pub fn set_current_task(data: &mut StoreData, today: &str, id: &Value) {
    roll_over_tasks_if_needed(data, today);
    for t in &mut data.tasks_today.tasks {
        t.is_current = task_id_eq(&t.id, id) && !t.done;
    }
}

pub fn delete_task(data: &mut StoreData, today: &str, id: &Value) {
    roll_over_tasks_if_needed(data, today);
    data.tasks_today.tasks.retain(|t| !task_id_eq(&t.id, id));
}

// —— 统计窗口数据包与历史操作(平移旧 options.js)——

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BadgesFull {
    pub badges: u32,
    pub current_streak: u32,
    pub goal: u32,
    pub unlocked_dates: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StatsBundle {
    pub stats: std::collections::BTreeMap<String, crate::store::StatEntry>,
    pub tasks_archive: std::collections::BTreeMap<String, Vec<Task>>,
    pub tasks_today: crate::store::TasksToday,
    pub badges: BadgesFull,
    pub notion_export_log: serde_json::Map<String, Value>,
    pub monthly_goals: Vec<GoalView>,
    pub today: String,
}

pub fn stats_bundle(data: &StoreData, today: &str) -> StatsBundle {
    let view = get_badges_view(data, today);
    StatsBundle {
        stats: data.stats.clone(),
        tasks_archive: data.tasks_archive.clone(),
        tasks_today: data.tasks_today.clone(),
        badges: BadgesFull {
            badges: view.badges,
            current_streak: view.current_streak,
            goal: view.goal,
            unlocked_dates: data.badges_state.unlocked_dates.clone(),
        },
        notion_export_log: data.notion_export_log.clone(),
        monthly_goals: goal_views(data, today),
        today: today.to_string(),
    }
}

/// 清零今日:只删当天 stats 计数,不动任务(旧 performClearToday)
pub fn clear_today_stats(data: &mut StoreData, today: &str) {
    data.stats.remove(today);
}

/// 历史明细里改任务完成态:今天走 tasksToday,其余日期改归档(旧 options setTaskDone)
pub fn set_history_task_done(data: &mut StoreData, today: &str, date: &str, id: &Value, done: bool) {
    if date == today {
        set_task_done(data, today, id, done);
        return;
    }
    if let Some(tasks) = data.tasks_archive.get_mut(date) {
        for t in tasks {
            if task_id_eq(&t.id, id) {
                t.done = done;
                t.done_override = Some(done);
                if done {
                    t.is_current = false;
                }
            }
        }
    }
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
pub struct DayStat {
    pub date: String,
    pub completed: u32,
    pub rotten: u32,
}

/// 从 6 天前到今天共 7 项(等价旧 GET_STATS / getLast7DaysStats)
fn last_7_days(data: &StoreData, today: &str) -> Vec<DayStat> {
    let Some(t) = parse_date(today) else { return vec![] };
    (0..7)
        .rev()
        .filter_map(|i| t.checked_sub_days(chrono::Days::new(i)))
        .map(|d| {
            let date = d.format("%Y-%m-%d").to_string();
            let e = data.stats.get(&date).copied().unwrap_or_default();
            DayStat { date, completed: e.completed, rotten: e.rotten }
        })
        .collect()
}

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
    pub monthly_goals: Vec<GoalView>,
    pub last_7_days: Vec<DayStat>,
    pub last_category: Option<String>,
    pub onboarding_done: bool,
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
        monthly_goals: goal_views(data, today),
        last_7_days: last_7_days(data, today),
        last_category: data.last_category.clone(),
        onboarding_done: data.onboarding_done,
        now,
    }
}

// —— 托盘标题(CONTENT.md 菜单栏节:专注 `24:59`,休息 `休息 4:32`,idle 无标题)——

pub fn tray_title(state: &TimerState, now: i64) -> Option<String> {
    let fmt_mmss = |ms: i64| {
        let total = (ms.max(0) + 999) / 1000;
        format!("{:02}:{:02}", total / 60, total % 60)
    };
    // 休息期红色提示符(ARCHITECTURE §3:"休息期间加红色提示符"——🔴 在菜单栏渲染为彩色)
    let fmt_break = |ms: i64| {
        let total = (ms.max(0) + 999) / 1000;
        format!("🔴 休息 {}:{:02}", total / 60, total % 60)
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
        assert!(roll_over_tasks_if_needed(&mut d, T));
        assert_eq!(d.tasks_today.date, T);
        assert!(d.tasks_today.tasks.is_empty());
        assert_eq!(d.tasks_archive["2026-07-16"].len(), 1);
    }

    /// tick 线程每秒都会调日切:同一天内反复调必须是零动作,
    /// 且不能把已归档的任务再塞一遍(v2 phase 01)
    #[test]
    fn task_rollover_is_idempotent_within_a_day() {
        let mut d = data_with_task();
        d.tasks_today.date = "2026-07-16".into();

        assert!(roll_over_tasks_if_needed(&mut d, T), "跨天第一次应真的归档");
        // 之后的每一拍都不应再报告归档
        for _ in 0..5 {
            assert!(!roll_over_tasks_if_needed(&mut d, T), "同一天内不应重复归档");
        }
        assert_eq!(d.tasks_archive["2026-07-16"].len(), 1, "归档不得重复堆积");
        assert!(d.tasks_today.tasks.is_empty());
    }

    /// 隔了多天没开机:中间的空白天不产生归档条目,当天任务照样清空
    #[test]
    fn task_rollover_across_multiple_days() {
        let mut d = data_with_task();
        d.tasks_today.date = "2026-07-10".into();
        assert!(roll_over_tasks_if_needed(&mut d, T)); // 直接跳到 07-17
        assert_eq!(d.tasks_archive.len(), 1);
        assert!(d.tasks_archive.contains_key("2026-07-10"));
        assert_eq!(d.tasks_today.date, T);
        assert!(d.tasks_today.tasks.is_empty());
    }

    /// 全新安装(date 为空)只把日期补上,不产生空归档条目
    #[test]
    fn task_rollover_on_fresh_install() {
        let mut d = StoreData::default();
        assert!(roll_over_tasks_if_needed(&mut d, T));
        assert_eq!(d.tasks_today.date, T);
        assert!(d.tasks_archive.is_empty(), "空任务不该产生归档条目");
        assert!(!roll_over_tasks_if_needed(&mut d, T));
    }

    // —— 月度目标(v2 phase 03)——

    fn data_with_goal() -> StoreData {
        let mut d = StoreData::default();
        d.tasks_today.date = T.into();
        add_goal(&mut d, 1000, "内参阅读", Some("学习".into()), 30.0);
        d
    }

    fn goal_id(d: &StoreData) -> String {
        d.monthly_goals[0].id.clone()
    }

    #[test]
    fn add_goal_normalizes_and_clamps() {
        let mut d = StoreData::default();
        add_goal(&mut d, 1000, "  ", Some("学习".into()), 30.0); // 空标题拒绝
        assert!(d.monthly_goals.is_empty());

        add_goal(&mut d, 1000, " 内参阅读 ", Some("不存在的分类".into()), f64::NAN);
        let g = &d.monthly_goals[0];
        assert_eq!(g.title, "内参阅读"); // trim
        assert_eq!(g.category.as_deref(), Some("工作")); // 非法分类归一化到默认
        assert_eq!(g.month_target, 1); // NaN → 1

        add_goal(&mut d, 2000, "内驱式学习", Some("学习".into()), 9999.0);
        assert_eq!(d.monthly_goals[1].month_target, 999); // clamp 上限
    }

    #[test]
    fn toggle_goal_creates_task_from_preset() {
        let mut d = data_with_goal();
        let id = goal_id(&d);

        let out = toggle_goal_today(&mut d, T, 5000, &id);
        assert!(out.ok && out.checked);
        let t = &d.tasks_today.tasks[0];
        assert_eq!(t.title, "内参阅读");
        assert_eq!(t.category.as_deref(), Some("学习")); // 分类来自目标预设
        assert_eq!(t.planned, 1); // 今天做几个在面板上敲,预设不决定
        assert_eq!(t.goal_id.as_deref(), Some(id.as_str()));
        assert!(t.is_current); // 没有其它当前任务时自动成为当前

        // 再点一次 = 取消,任务被删掉
        let out = toggle_goal_today(&mut d, T, 6000, &id);
        assert!(out.ok && !out.checked);
        assert!(d.tasks_today.tasks.is_empty());
    }

    #[test]
    fn toggle_goal_locked_after_first_tomato() {
        let mut d = data_with_goal();
        let id = goal_id(&d);
        toggle_goal_today(&mut d, T, 5000, &id);
        d.tasks_today.tasks[0].used = 1; // 已投入一个番茄

        let out = toggle_goal_today(&mut d, T, 6000, &id);
        assert!(!out.ok);
        assert_eq!(out.reason.as_deref(), Some("locked"));
        assert!(out.checked);
        assert_eq!(d.tasks_today.tasks.len(), 1, "拒绝取消时任务不得被删");
    }

    #[test]
    fn toggle_unknown_goal_is_noop() {
        let mut d = data_with_goal();
        let out = toggle_goal_today(&mut d, T, 5000, "g_nonexistent");
        assert!(!out.ok);
        assert_eq!(out.reason.as_deref(), Some("not-found"));
        assert!(d.tasks_today.tasks.is_empty());
    }

    #[test]
    fn goal_month_progress_counts_only_current_month() {
        let mut d = data_with_goal();
        let id = goal_id(&d);
        toggle_goal_today(&mut d, T, 5000, &id);
        d.tasks_today.tasks[0].used = 2; // 今天 2 个

        // 本月早些时候归档的 3 个
        d.tasks_archive.insert(
            "2026-07-02".into(),
            vec![Task { id: Value::from(9), goal_id: Some(id.clone()), used: 3, ..Task::default() }],
        );
        // 上个月的 7 个,不该计入
        d.tasks_archive.insert(
            "2026-06-20".into(),
            vec![Task { id: Value::from(8), goal_id: Some(id.clone()), used: 7, ..Task::default() }],
        );
        // 本月但属于别的目标,不该计入
        d.tasks_archive.insert(
            "2026-07-03".into(),
            vec![Task { id: Value::from(7), goal_id: Some("g_other".into()), used: 5, ..Task::default() }],
        );

        let v = &goal_views(&d, T)[0];
        assert_eq!(v.month_done, 5); // 2 + 3
        assert_eq!(v.month_target, 30);
        assert!(v.checked_today);
        assert!(v.locked); // used > 0
    }

    #[test]
    fn update_goal_renames_todays_task_too() {
        let mut d = data_with_goal();
        let id = goal_id(&d);
        toggle_goal_today(&mut d, T, 5000, &id);

        update_goal(&mut d, &id, Some("内参精读".into()), Some("工作".into()), Some(50.0));
        assert_eq!(d.monthly_goals[0].title, "内参精读");
        assert_eq!(d.monthly_goals[0].month_target, 50);
        // 面板上两处名字必须一致
        assert_eq!(d.tasks_today.tasks[0].title, "内参精读");
        assert_eq!(d.tasks_today.tasks[0].category.as_deref(), Some("工作"));

        // None 表示不动
        update_goal(&mut d, &id, None, None, None);
        assert_eq!(d.monthly_goals[0].title, "内参精读");
        assert_eq!(d.monthly_goals[0].month_target, 50);
    }

    #[test]
    fn delete_goal_keeps_existing_records() {
        let mut d = data_with_goal();
        let id = goal_id(&d);
        toggle_goal_today(&mut d, T, 5000, &id);
        d.tasks_today.tasks[0].used = 2;

        delete_goal(&mut d, &id);
        assert!(d.monthly_goals.is_empty());
        // 删目标不抹已有记录
        assert_eq!(d.tasks_today.tasks.len(), 1);
        assert_eq!(d.tasks_today.tasks[0].used, 2);
    }

    #[test]
    fn set_task_planned_clamps_to_used() {
        let mut d = data_with_task(); // planned 4, used 0
        let id = d.tasks_today.tasks[0].id.clone();

        set_task_planned(&mut d, T, &id, 8.0);
        assert_eq!(d.tasks_today.tasks[0].planned, 8);

        d.tasks_today.tasks[0].used = 3;
        set_task_planned(&mut d, T, &id, 2.0); // 想减到 2,但已做 3
        assert_eq!(d.tasks_today.tasks[0].planned, 3, "不得小于已投入数");

        set_task_planned(&mut d, T, &id, 500.0);
        assert_eq!(d.tasks_today.tasks[0].planned, 99); // clamp 上限

        set_task_planned(&mut d, T, &id, f64::NAN);
        assert_eq!(d.tasks_today.tasks[0].planned, 3); // NaN → 回下限
    }

    #[test]
    fn add_task_rules() {
        let mut d = StoreData::default();
        d.tasks_today.date = T.into();
        add_task(&mut d, T, 1000, "  ", None, 3.0); // 空标题拒绝
        assert!(d.tasks_today.tasks.is_empty());
        add_task(&mut d, T, 1000, "写周报", Some("学习".into()), 99.0);
        let t0 = &d.tasks_today.tasks[0];
        assert_eq!(t0.planned, 20); // clamp 1..20
        assert!(t0.is_current); // 首个任务自动成为当前
        assert_eq!(d.last_category.as_deref(), Some("学习"));
        add_task(&mut d, T, 2000, "读书", Some("不存在的分类".into()), f64::NAN);
        let t1 = &d.tasks_today.tasks[1];
        assert_eq!(t1.category.as_deref(), Some("工作")); // 非法分类回默认
        assert_eq!(t1.planned, 1); // NaN → 1
        assert!(!t1.is_current); // 已有当前任务,不抢
    }

    #[test]
    fn task_done_and_current() {
        let mut d = StoreData::default();
        d.tasks_today.date = T.into();
        add_task(&mut d, T, 1000, "A", None, 1.0);
        add_task(&mut d, T, 2000, "B", None, 1.0);
        let id_a = d.tasks_today.tasks[0].id.clone();
        let id_b = d.tasks_today.tasks[1].id.clone();
        set_task_done(&mut d, T, &id_a, true);
        assert!(d.tasks_today.tasks[0].done);
        assert_eq!(d.tasks_today.tasks[0].done_override, Some(true));
        assert!(!d.tasks_today.tasks[0].is_current); // 完成即让出当前
        set_current_task(&mut d, T, &id_b);
        assert!(d.tasks_today.tasks[1].is_current);
        set_current_task(&mut d, T, &id_a); // 已完成任务不能设为当前
        assert!(!d.tasks_today.tasks[0].is_current);
        delete_task(&mut d, T, &id_b);
        assert_eq!(d.tasks_today.tasks.len(), 1);
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
        assert_eq!(tray_title(&s, 0).unwrap(), "🔴 休息 4:32");
    }

    #[test]
    fn last_minute_fires_once_per_end_time() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        start_focus(&mut d, now, &DUR);
        let end = d.timer_state.end_time.unwrap();
        let mut fired = None;
        // 还没进最后一分钟窗口
        assert!(check_last_minute(&d, end - 90_000, &DUR, &mut fired).is_empty());
        // 进窗口:触发一次
        let effects = check_last_minute(&d, end - 30_000, &DUR, &mut fired);
        assert!(effects.contains(&Effect::SoftNudge));
        // 同一 endTime 不再触发
        assert!(check_last_minute(&d, end - 20_000, &DUR, &mut fired).is_empty());
        // 关掉开关:静默但仍记为已处理
        d.settings.last_minute_enabled = false;
        let mut fired2 = None;
        assert!(check_last_minute(&d, end - 30_000, &DUR, &mut fired2).is_empty());
        assert_eq!(fired2, Some(end));
    }

    #[test]
    fn break_entry_picks_subtitle_and_pins_panel() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        d.stats.entry(T.into()).or_default().completed = 1;
        let effects = start_break(&mut d, now, T, &DUR);
        let subtitle = d.timer_state.break_subtitle.clone().unwrap();
        assert!(MONSTER_SUBTITLES.contains(&subtitle.as_str()));
        // 休息开始:面板钉住 + 白噪音
        assert!(effects.contains(&Effect::PanelPin));
        assert!(effects.contains(&Effect::WhiteNoiseStart));
    }

    #[test]
    fn break_end_unpins_panel() {
        let mut d = StoreData::default();
        let now = 1_000_000_000;
        d.stats.entry(T.into()).or_default().completed = 1;
        start_break(&mut d, now, T, &DUR);
        let end = d.timer_state.end_time.unwrap();
        let effects = handle_phase_end_if_due(&mut d, end, T, &DUR);
        assert!(effects.contains(&Effect::PanelUnpin));
    }
}
