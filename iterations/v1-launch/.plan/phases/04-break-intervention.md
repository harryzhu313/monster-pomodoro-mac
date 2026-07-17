# Phase 04 — 到点干预：音频、钉住面板与休息流

**Status**: `completed`
**目标**: 到点干预组合完整落地（chime → 面板自动弹出钉住 → 托盘变红 → 白噪音），加时扣配额闭环，产品红线 1 成立
**前置**: Phase 03（主面板可用）

> ⚠️ 2026-07-17 方案变更（用户决策）：系统通知三处全删，改为**休息开始面板自动弹出并钉住**
> （失焦不隐/托盘点击不收起/不抢键盘焦点，休息结束自动收起）。原通知实现已移除，
> tauri-plugin-notification 依赖已卸载，PRD/CONTENT/ARCHITECTURE 已同步。

## 验收判据

- `TOMATO_TEST=1` 全周期：到点 chime + **面板自动弹出钉住** + 托盘 🔴 + 白噪音（延迟 1 秒起播、淡入淡出）完整触发
- 钉住期间：点面板外不消失、点托盘图标不消失；休息结束/加时成功自动收起；弹出不抢键盘焦点
- 休息期间面板**唯一出口是加时（扣配额）**；配额用尽时只能等待——遍历所有入口确认无免费跳过（产品红线 1）
- 最后一分钟提示（纯声音）可用且可在设置关闭
- 白噪音行为对齐旧 `offscreen/offscreen.js`

## Tasks

- [x] `audio.rs`：chime 双音正弦合成；白噪音噪声源 + 音量包络（淡入淡出），settings 控制开关 (audio.rs 重写为常驻引擎:专用线程 + mpsc 通道;棕噪音无限循环源,800ms 淡入/400ms 淡出经 Sink 音量;E5→B5 chime + G5→C6 nudge;开关裁决在 lib.rs run_effects)
- [x] 休息开始序列：chime → 系统通知 → 白噪音延迟 1 秒起播；休息结束白噪音淡出停止 (NoiseStart{delay:1s} 引擎内排期;NoiseStop 阻塞式淡出保证 chime 不与噪音尾巴重叠,等价旧 offscreen"等淡出完成再回应";实测日志顺序正确)
- [-] `tauri-plugin-notification` 接入 + 权限未授予时的降级处理 (skipped 2026-07-17: 用户决策系统通知全删,改为钉住面板;已实现的通知代码与依赖整体移除)
- [x] 休息开始面板自动弹出并钉住;休息结束/加时成功自动收起;不抢键盘焦点 (added 2026-07-17: 替代系统通知的用户方案;timer.rs Effect::PanelPin/PanelUnpin + lib.rs panel_pinned AtomicBool 控制 blur/托盘 toggle;overtime 成功附带停白噪音+收面板;重启恢复到休息态自动重新钉住;单测 break_entry_picks_subtitle_and_pins_panel / break_end_unpins_panel)
- [x] 面板窗口 visibleOnAllWorkspaces + focus:false (added 2026-07-17: 钉住面板跨桌面空间可见;全屏 Space 覆盖为已知限制,用户已接受,PRD §5 已记)
- [x] 托盘休息态：红色提示符 + `休息 m:ss` 标题格式（CONTENT.md 菜单栏节）(timer.rs tray_title:`🔴 休息 m:ss`,ARCHITECTURE §3"红色提示符"用 🔴 emoji 实现——菜单栏 emoji 渲染彩色;单测已更新)
- [x] 最后一分钟提示（纯提示音，可关；通知横幅已随方案变更移除）(timer.rs check_last_minute,按 endTime 去重;独立设置 lastMinuteEnabled 默认开;单测 last_minute_fires_once_per_end_time)
- [x] 加时流：面板加时选项（+5 / +25 / 自定义 0.1–120 分钟支持小数，对齐旧 lockscreen.js）、扣配额、配额尽则禁用 (panel.html extend 区 + panel.js claim 逻辑;预设按钮 TEST_MODE 分钟当秒、自定义始终真分钟,均对齐旧实现;配额尽:标题变"今日配额已用完"+全按钮禁用)
- [x] 休息期面板副标题：从 `MONSTER_SUBTITLES` 池随机挑一句 (8 句文案池平移至 timer.rs const;进入休息时挑一句存 timerState.breakSubtitle;default 主题回落旧静态文案)
- [x] 红线自检：遍历托盘 / 面板 / 预留快捷键入口，确认休息期无免费跳过出口 (见 Notes"红线审计")

## Notes

### 红线审计（休息期无免费跳过出口，2026-07-17）

| 入口 | 状态 |
|---|---|
| 托盘点击 | 只 toggle 面板显示，无状态操作；**钉住期间点托盘不收起** ✅ |
| 点面板外（blur） | 钉住期间失焦不隐藏（panel_pinned 原子位控制） ✅ |
| 面板主按钮 | BREAKING 下"休息锁定中"disabled，且 dataset.action 为空 ✅ |
| 面板放弃/重置 | UI disabled；**Rust 侧双保险**：`abandon` 对 phase≠focus 无效、`reset(force=false)` 在 break 中无效（单测 no_free_exit_from_break 覆盖） ✅ |
| 加时 | 唯一出口，必扣配额，配额尽即禁用（Rust quota-exhausted 双保险） ✅ |
| 全局快捷键 | 尚未接入（phase 07），接入时快捷键只映射 start/pause，不映射 reset/abandon ⚠️ 到时复查 |

### 验收与提交

- 用户验收：2026-07-17 通过（含方案变更后的钉住面板 5 项测试）
- Commit：（提交后回填）

### 自测记录（2026-07-17）

- `cargo test` 22 passed（新增 last_minute_fires_once_per_end_time / break_entry_picks_subtitle）
- TOMATO_TEST 实跑 58 秒双周期：nudge+通知(T-5s) → chime+随机文案通知+白噪音起 → 30s → 白噪音停+chime+自动下轮 → 第二轮 nudge 再触发（endTime 去重正确）
- 通知经 plugin 发送无报错；**dev 裸二进制的通知横幅是否真实弹出待用户确认**（macOS 对非 bundle 二进制可能不显示，若不显示需 `cargo tauri build` 出 .app 验证）

### 实现决策

- 旧版"chime 后 sleep 1s 再切状态"→ 状态即时切换，错峰由音频引擎实现：NoiseStart 带 1s 延迟排期、NoiseStop 阻塞式淡出（后续 chime 自然排在淡出之后）
- 白噪音状态驱动同步（等价旧 syncWhiteNoise）：update_settings 关开关立即停/起；重启恢复到 BREAKING 时自动续播
- 面板窗口高 420→480（休息态多出副标题 + 加时区块）
- 旧 lockscreen 的 afraid monster hover 彩蛋（悬浮加时按钮跳出"确定吗?"）未平移——PRD 主面板槽位未列，如想要归 phase 07 polish
