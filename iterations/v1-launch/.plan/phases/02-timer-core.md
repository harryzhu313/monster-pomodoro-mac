# Phase 02 — Rust 核心：状态机与持久化

**Status**: `not started`
**目标**: `timer.rs` 状态机行为对齐旧 service-worker.js，`store.rs` 原子持久化，TOMATO_TEST 模式全周期跑通
**前置**: Phase 01（骨架可运行，spike 结论已出）

## 验收判据

- `TOMATO_TEST=1 cargo tauri dev` 全周期跑通：专注（15s）→ 到点 → 休息（30s）→ 自动开始下一轮
- 暂停 / 恢复 / 放弃 / 加时扣配额 / 配额日切重置 / 长休息每 4 番茄触发，逐项对齐旧插件行为清单（见 Notes）
- 杀进程重启后，进行中的倒计时从 store.json 恢复且无漂移（endTime 制）
- Mac 睡眠唤醒 / 系统时间变化后，倒计时仍以 endTime 为准

## Tasks

- [ ] 读旧 `service-worker.js`（1258 行），梳理状态机行为清单（状态、转换、防御规则）写入 Notes
- [ ] `timer.rs`：状态机 `idle / focusing / breaking / paused`，基于 endTime 时间戳（不累加 tick）
- [ ] 配额结算：每日 3 次加时、日切重置；长休息：每 N 个番茄（默认 4）触发、时长可配（默认 20 分钟）
- [ ] `store.rs`：`store.json` 原子写（临时文件 + rename），schema 顶层键沿用旧插件（`timerState / quotaState / settings / stats / tasksToday / tasksArchive / badgesState / lastCategory / notionExportLog`）
- [ ] command：`start / pause / resume / reset / abandon / overtime / update_settings`
- [ ] `state-update` 事件广播（状态快照，含 TEST 模式下发的时长配置）
- [ ] `TOMATO_TEST=1` 单一开关（专注 15s / 休息 30s，只在 Rust 侧）
- [ ] 托盘标题接入真实状态：专注 `mm:ss` / 休息红色提示符 + `休息 m:ss` / idle 无标题
- [ ] 重启恢复：启动时从 store.json 恢复进行中状态

## Notes

（旧插件状态机行为清单、移植时的疑问与查证记录写这里）
