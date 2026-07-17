# Phase 02 — Rust 核心：状态机与持久化

**Status**: `completed`
**目标**: `timer.rs` 状态机行为对齐旧 service-worker.js，`store.rs` 原子持久化，TOMATO_TEST 模式全周期跑通
**前置**: Phase 01（骨架可运行，spike 结论已出）

## 验收判据

- `TOMATO_TEST=1 cargo tauri dev` 全周期跑通：专注（15s）→ 到点 → 休息（30s）→ 自动开始下一轮
- 暂停 / 恢复 / 放弃 / 加时扣配额 / 配额日切重置 / 长休息每 4 番茄触发，逐项对齐旧插件行为清单（见 Notes）
- 杀进程重启后，进行中的倒计时从 store.json 恢复且无漂移（endTime 制）
- Mac 睡眠唤醒 / 系统时间变化后，倒计时仍以 endTime 为准

## Tasks

- [x] 读旧 `service-worker.js`（1258 行），梳理状态机行为清单（状态、转换、防御规则）写入 Notes (见下方 Notes"行为清单",2026-07-17 全文通读)
- [x] `timer.rs`：状态机 `idle / focusing / breaking / paused`，基于 endTime 时间戳（不累加 tick）(src-tauri/src/timer.rs:1-460,含 14 个单元测试;`cargo test` 18 passed)
- [x] 配额结算：每日 3 次加时、日切重置；长休息：每 N 个番茄（默认 4）触发、时长可配（默认 20 分钟）(timer.rs get_quota/consume_quota/should_take_long_break;单测 quota_lazy_day_reset / quota_exhaustion / long_break_trigger / fourth_tomato_gets_long_break)
- [x] `store.rs`：`store.json` 原子写（临时文件 + rename），schema 顶层键沿用旧插件 (src-tauri/src/store.rs:200-240;实测文件权限 600,顶层键单测覆盖;损坏文件侧移 .corrupt 不覆盖)
- [x] command：`start / pause / resume / reset / abandon / overtime / update_settings` + `reset_quota` (src-tauri/src/lib.rs:88-142)
- [x] `state-update` 事件广播（状态快照，含 TEST 模式下发的时长配置）(lib.rs sync_out + tick 线程每秒广播;快照含 timer/quota/settings/durations/todayStats/badges/tasksToday)
- [x] `TOMATO_TEST=1` 单一开关（专注 15s / 休息 30s，只在 Rust 侧）(timer.rs durations();实测日志 `[TOMATO_TEST] 测试模式`)
- [x] 托盘标题接入真实状态：专注 `mm:ss` / 休息 `休息 m:ss` / idle 无标题 (timer.rs tray_title + 单测 tray_title_formats;托盘变红是 phase 04;视觉待用户验收)
- [x] 重启恢复：启动时从 store.json 恢复进行中状态 (实测:BREAKING 中杀进程,20 秒后重启 → 第一拍补结算过期休息并自动开始下一轮,剩余时间准确)

## Notes

### 行为清单（源自旧 service-worker.js 全文通读，2026-07-17）

**状态形状**（timerState，字段名沿用）：
- `state`: IDLE | FOCUSING | BREAKING | PAUSED；`phase`: null|focus|break；`breakKind`: null|short|long
- `endTime`(ms 时间戳) / `pausedRemaining` / `prePauseState` / `focusStartedAt` / `loveMonsterUntil`

**转换规则**：
- `start`：任意 → FOCUSING，endTime = now + 25min，focusStartedAt = now (sw:343)
- `pause`：**仅 FOCUSING 可暂停**（break 不可暂停），记 pausedRemaining (sw:586)
- `resume`：仅 PAUSED，endTime = now + pausedRemaining，回 prePauseState (sw:600)
- `reset`：**BREAKING / PAUSED-break 期间无效**（无免费跳过，红线 1）；force 变体供内部用 (sw:614)
- `abandon`：仅 phase==focus 生效；**启动后 10 秒宽限期内放弃不记烂番茄**（误点保护）；否则 stats.rotten+1 + 当前任务 rotten+1 (sw:622-636)
- `overtime(ms)`：仅 BREAKING/FOCUSING；扣配额（尽则拒绝 quota-exhausted）；→ FOCUSING，endTime = now+ms，focusStartedAt 重置；**加时跑完走正常专注完成路径（completed+1）** (sw:316-341)

**专注到点**（sw:381-390）：chime → stats.completed+1 → 当前任务 used+1 → 等 1s → startBreak(long? = completedToday>0 && completedToday % every==0 && longBreakEnabled)

**休息到点**（sw:393-408）：停白噪音(等淡出) → 120ms → chime → 1s → autoStartNextFocus ? 通知+startFocus : 通知+reset(force)

**startBreak**（sw:474-503）：先 recordBreakEntryMilestone（徽章结算）→ BREAKING；milestone 命中则 loveMonsterUntil = now+15s

**配额**（sw:268-312）：每日 3 次，惰性日切（存 {date,used}，读时不匹配今天即视为 0）；used 读时 clamp 0..3；**消费配额同时标记 badgesState.lastExtendDate = today（连击变脏）**

**长休息**：longBreakMinutes clamp 15..30（默认 20）；longBreakEvery clamp 2..12（默认 4）；TEST_MODE 下长休息=短休息时长

**连击/徽章**（sw:779-900）：streak 基点 = max(anchor 前一天, lastExtendDate, 最近解锁日)，终点 = 今天有完成番茄?今天:昨天，clamp 0..7；今日 quotaUsed>0 → streak=0；**颁发时机 = 今天第 1 个番茄进入休息 && streak≥7 && 配额干净 && 今天未解锁过**

**统计**（sw:652-707）：stats[date]={completed,rotten}，旧数字格式读时归一化；保留 366 天供热力图

**任务**（sw:709-764）：tasksToday={date,tasks[]}，跨天自动归档到 tasksArchive（按 id 去重合并）；"当前任务" = isCurrent && !done

**phase-end 幂等防御**（sw:357-371）：handledEndTime + 10s stale 窗口，防止重复结算

**TEST_MODE**（sw:8-14）：专注 15s / 休息 30s / 最后一分钟提示 5s；重载清配额；新版换成 `TOMATO_TEST=1` 环境变量单一开关

**settings 默认值**（sw:37-46）：autoStartNextFocus:true / whiteNoiseEnabled:true / chimeEnabled:true / longBreakEnabled:true / longBreakEvery:4 / longBreakMinutes:20 / theme:'default'（lockscreenBg 为浏览器特有，弃）

**phase 02 明确不做**（留给后续 phase）：音频联动与通知（04）、最后一分钟提示（04）、锁屏注入→已废弃、悬浮窗→已废弃、Notion（06）、庆祝弹层 UI（07）

**待确认**：托盘 PAUSED 态标题格式 CONTENT.md 未定义（旧版 badge 黄色 + "已暂停"）——暂用 `⏸ mm:ss`，用户验收时确认

### 验收与提交

- 用户验收：2026-07-17 通过
- Commit：`ceef90e`（用户委托 AI 执行并推送）

### 自测记录（2026-07-17）

- `cargo test`：18 passed / 0 failed（含状态机 14 项：配额日切、耗尽拒绝、长休息触发点、clamp 边界、连击计算、里程碑颁发、放弃宽限期、休息无免费出口红线、加时流、专注结算、自动/手动下一轮、暂停恢复、任务归档、托盘标题格式）
- 真实周期（TOMATO_TEST=1 + TOMATO_AUTOSTART=1 钩子）：专注 15s → Chime+通知+白噪音起 → 休息 30s → 白噪音停+Chime+通知 → 自动下一轮，日志与 store.json 全对
- 重启恢复：BREAKING 中 kill，endTime 过期后重启 → 启动第一拍补结算 + 自动开始下一轮（等价旧 reconcileTimerAfterWake）
- store.json 权限实测 `-rw-------`（600）；测试数据已清除

### 实现层决策（对齐时的有意偏差）

- 旧版 chime 后 `sleep(1000)` 再切状态（音频错峰）→ 新版状态即时切换，音频错峰归 phase 04 效果执行器——状态机不睡觉
- 旧 SW 的 handledEndTime 幂等防御（防 alarm 竞态）→ 新版单进程单 tick 驱动，天然无竞态，不移植
- 新增 `TOMATO_AUTOSTART=1` 开发钩子：启动即开始专注，供无 UI 时观察完整周期（类比旧 TEST_MODE 惯例）
- 新增依赖 chrono（用户 2026-07-17 批准），已登记 ARCHITECTURE §1
