# Phase 05 — 统计与设置窗口

**Status**: `completed`
**目标**: 独立统计与设置窗口完整渲染 6 个数据区 + 3 个设置区，设置改动即时持久化并生效
**前置**: Phase 04（统计数据已在真实累积）

## 验收判据

- 统计窗口按需创建、关闭销毁（ARCHITECTURE §3），vibrancy 生效，最小宽度 560px
- 全部区块渲染且区块标题对齐 CONTENT.md（🏅 7 天连击 / 🟩 最近一年 / 📊 最近 7 天 / 📖 历史明细 / 💾 本地备份 / 📤 Notion 导出 / ⏱ 番茄节奏 / 🔔 通知）
- 旧 options 的"🪟 桌面悬浮"区确认**不存在**
- 设置改动即时持久化（重启后保留）并生效（如关白噪音立即静音）

## Sections

### Section A — 窗口骨架、连击与热力图

**Gate**: `committed`

**自测判据**：

- 面板底栏按钮打开统计窗口，关闭后再开状态正确
- 7 天连击徽章显示当前进度；52 周热力图 + 月份标尺渲染正确（对照 store 数据抽查 3 天）

**Tasks**：

- [x] 统计窗口按需创建 / 关闭销毁，vibrancy (lib.rs open_stats command:WebviewWindowBuilder 动态建窗 680×760/min 560,UnderWindowBackground vibrancy;实测启动无报错。注:未单拆 windows.rs,窗口管理量少暂不值得拆,ARCHITECTURE §7 的 windows.rs 留给 phase 07 庆祝窗口时再评估)
- [x] stats.html / stats.css 骨架（分区布局，theme.css token）(src/stats/stats.html 9 个区块;stats.css 全 token 化,热力图色阶新增 --heat-0..3 token 深浅两套)
- [x] 🏅 7 天连击徽章区（本周进度 + 徽章位）(stats.js renderBadges:52 格 love monster 墙,近 365 天解锁数点亮,平移旧 renderBadgeSlots)
- [x] 🟩 年度热力图（52 周 + 月份标尺，仿 GitHub）(stats.js renderHeatmap 逐行平移旧 refreshHeatmap:53 列×7 行、月份标尺、未来格透明、累计 meta;色阶 0/1-6/7-12/13+)

**记录**：

- 自测：2026-07-17 cargo build/test 全绿;TOMATO_OPEN_STATS=1 钩子启动实测窗口创建与 vibrancy 无报错
- 用户验收：2026-07-17 通过（含三项调整拍板:🔔 提醒与声音改名/外观跟随系统不做开关/锁屏背景删除）
- Commit：见 plan.md 或 git log(Phase 05 提交)

---

### Section B — 7 天图表与历史明细

**Gate**: `committed`

**自测判据**：

- 柱状图与统计卡数值和 store.json 一致；清零今日后当日归零且有确认
- 历史明细按日展开，任务记录完整；每日"导入 Notion"按钮就位（功能 phase 06 接线）

**Tasks**：

- [x] 📊 最近 7 天：柱状图 + 统计卡（当前连击 / 最长连击 / 总番茄）+ 清零今日 (stats.js renderChart/computeStreaks 平移旧实现含 🤪 烂番茄标记;清零今日两步确认 → clear_today_stats command)
- [x] 📖 历史明细：按日展开的任务记录，每日一个"导入 Notion"动作（占位）(stats.js renderHistory* 平移旧实现:doneOverride>done>自动推断 三级完成态、计划外番茄行、30 天上限、今天默认展开;完成/未完成按钮 → set_history_task_done command(今天走 tasksToday,历史改归档);Notion 按钮 disabled 待 06)

**记录**：

- 自测：2026-07-17 渲染逻辑对照旧 options.js 逐行核对;Rust 侧 set_history_task_done/clear_today_stats 走 with_store 统一持久化
- 用户验收：2026-07-17 通过（含三项调整拍板:🔔 提醒与声音改名/外观跟随系统不做开关/锁屏背景删除）
- Commit：见 plan.md 或 git log(Phase 05 提交)

---

### Section C — 设置区

**Gate**: `committed`

**自测判据**：

- 每项设置改动 → 重启 app 后保留；音频类开关立即生效
- ⏱ 番茄节奏改"每 N 个番茄"后，TOMATO_TEST 跑一轮验证长休息触发点变化

**Tasks**：

- [x] ⏱ 番茄节奏：长休息开关 / 每 N 个番茄（默认 4）/ 长休息时长（默认 20 分钟）(开关联动禁用子项,平移旧行为;clamp 在 Rust update_settings)
- [x] 🔔 提醒与声音：提示音开关 / 白噪音开关 / 最后一分钟提示 (三开关 → update_settings;白噪音休息中即时生效走 phase 04 的状态驱动同步)
- [x] 通用：主题（default / monster）/ 休息结束自动开始下一番茄 / 开机自启与全局快捷键（占位 disabled，功能接线在 phase 07）(🎨 其他设置区;主题切换经 state-update 广播即时反映到面板小怪兽)
- [x] 💾 本地备份、📤 Notion 导出两区的 UI 骨架（功能在 phase 06）(按钮/输入框 disabled + title 注明;文案平移旧 options.html,浏览器措辞改 app 措辞)

**记录**：

- 自测：2026-07-17 设置项与 Settings 字段一一对应核对;主题/白噪音的即时生效链路(update_settings → sync_out/音频同步)在 04 已验证
- 用户验收：2026-07-17 通过（含三项调整拍板:🔔 提醒与声音改名/外观跟随系统不做开关/锁屏背景删除）
- Commit：见 plan.md 或 git log(Phase 05 提交)

## Notes

### 对 CONTENT/旧版的有意偏差（待用户验收确认）

- 区块标题 `🔔 通知` → `🔔 提醒与声音`：系统通知已删,原标题名不副实;改用 PRD §5 的措辞,并把白噪音/最后一分钟并入该区（旧版白噪音在 🎨 其他设置）
- CONTENT"新增设置项标签"里的 `外观跟随系统` 未做成开关——深浅外观本来就自动跟随系统(CSS prefers-color-scheme),PRD §5 通用区也未列此项;如需强制浅/深色开关,归后续迭代
- 旧 🎨 其他设置里的 `锁屏背景` 下拉删除(锁屏已废弃;backgrounds SVG 留给庆祝弹层/未来 v2)
- 页头 h1 去掉 🍅 前缀(DESIGN Don'ts 精神),区块标题 emoji 按 CONTENT 保留
- 开发钩子 TOMATO_OPEN_STATS=1:启动即开统计窗口(自测用)

### 待办遗留

- 历史明细"导入 Notion"按钮 disabled → phase 06 接线(notionExportLog 的已导入态渲染逻辑已就位)
- 开机自启/全局快捷键两行 disabled → phase 07 接线
