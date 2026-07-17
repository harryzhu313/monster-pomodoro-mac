# Phase 03 — 主面板

**Status**: `completed`
**目标**: 主面板 UI 完成日常操作闭环（计时 / 任务 / 收获 / 配额），PRD 场景 1"3 秒开始专注"成立
**前置**: Phase 02（状态机与 state-update 事件可用）

## 验收判据

- 点托盘 → 添加/选中任务 → 点"开始专注"，3 秒内进入专注（场景 1）
- 面板所有状态显示由 `state-update` 驱动，前端零自维护计时状态（AGENTS 规则 6）
- 文案对齐 CONTENT.md 主面板节（📦 平移旧 popup，不"优化"）
- theme.css 深浅两套 token 生效，深浅模式切换正常；除 theme.css 外 grep 不到硬编码 hex

## Sections

### Section A — 主题与素材基座

**Gate**: `committed`（A 无独立可见面，与 B/C 合并一次用户验收）

**自测判据**：

- `cargo tauri dev` 面板加载 theme.css 无 404，深浅模式下背景 / 文字色正确切换
- `grep -rn '#[0-9a-fA-F]\{3,6\}' src/ --include='*.css' --include='*.js'` 仅命中 `shared/theme.css`
- `src/shared/monsters/` 有 5 只小怪兽 SVG

**Tasks**：

- [x] 按 DESIGN.md YAML token 手写 `src/shared/theme.css`（深浅两套 CSS 变量）(src/shared/theme.css:1-100;新增 --primary-text 深浅切换、分类徽章四色 token 化含深色变体)
- [x] 从旧仓库复制素材：`themes/monster/*.svg` → `src/shared/monsters/`，`backgrounds/*.svg` → `src/shared/backgrounds/` (5 只小怪兽 + 3 张背景已就位)
- [x] `src/shared/util.js`：日期、mm:ss 格式化等共享工具 (formatMs/computeRemaining/分类常量/MONSTER_BY_STATE)

**记录**：

- 自测：2026-07-17 grep 仅命中 theme.css;浏览器 mock 预览深浅两模式截图核对通过
- 用户验收：2026-07-17 通过（含默认主题/休息文案/暂停托盘格式三项拍板）
- Commit：见 Notes「验收与提交」

---

### Section B — 计时卡

**Gate**: `committed`

**自测判据**：

- `TOMATO_TEST=1` 走一轮：小怪兽表情、阶段文案、倒计时、主按钮全程与状态同步
- 放弃 → 记一颗烂番茄；重置 → 回 idle；配额显示"今日剩 N/3"

**Tasks**：

- [x] 计时卡布局：小怪兽表情 / 阶段文案 / 倒计时 / 主按钮（开始·暂停）(src/panel/panel.html:11-32, panel.js renderTimer 平移旧 renderTimer 全部状态分支含长休息/宽限期文案)
- [x] 放弃（记烂番茄，带确认提示 title）/ 重置 (panel.html:17-20;禁用规则对齐旧版:休息中全禁)
- [x] 今日配额显示（剩 N/3）；加时按钮（仅休息中可见，具体加时流在 phase 04）(panel.js:renderTimer 控制 is-hidden;按钮暂 disabled 待 04 接流)
- [x] 订阅 `state-update` 渲染、command 发指令的前端 IPC 封装 (panel.js:19-34;无 __TAURI__ 时降级 mock 供浏览器预览)

**记录**：

- 自测：2026-07-17 浏览器 mock 校验 IDLE 渲染;状态分支逻辑对照旧 popup.js 逐行核对;真实状态流转依赖 Rust 广播(phase 02 已验)
- 用户验收：2026-07-17 通过（含默认主题/休息文案/暂停托盘格式三项拍板）
- Commit：见 Notes「验收与提交」

---

### Section C — 任务、收获与底栏

**Gate**: `committed`

**自测判据**：

- 添加任务（名称 / 分类四选一 / 计划番茄数）→ 列表出现；选中任务开始专注 → 完成后该任务计数 +1
- 空状态显示"还没有任务，先加一个吧。"，输入占位符"比如：内参阅读"
- 今日收获行：🍅 成熟 / 🥫 烂番茄 / 🔥 连击天数正确显示

**Tasks**：

- [x] 任务添加表单（任务名 / 分类：工作·学习·生活·兴趣爱好 / 计划番茄数）(panel.html:37-52;分类记忆 lastCategory 平移旧行为)
- [x] 任务列表（当前专注标记、每任务完成计数、删除）(panel.js renderTasks/renderTaskTomatoes 平移旧实现含 +N 折叠与"超N"标记;仅内容变化时重建避免打断输入)
- [x] 今日收获行（🍅 / 🥫 图标行 + 🔥 已连续 N 天）(panel.js renderHarvest;连续天数按旧 popup.js:242-252 从 last7Days 计算)
- [x] 底栏：打开统计与设置窗口按钮（窗口本体 phase 05）+ 提示语 (panel.html:55-59;按钮暂 disabled;去掉 ⚙ emoji 遵守 DESIGN Don'ts)
- [x] 任务数据走 Rust command 持久化（`tasksToday` 键）(timer.rs add_task/set_task_done/set_current_task/delete_task + lib.rs 4 个 command;cargo test 20 passed 含 2 个任务行为新测试)

**记录**：

- 自测：2026-07-17 mock 快照含 2 任务(当前/已完成)渲染核对;Rust 侧任务规则单测(空标题拒绝/clamp 1..20/非法分类回默认/首任务自动当前/完成让出当前)
- 用户验收：2026-07-17 通过（含默认主题/休息文案/暂停托盘格式三项拍板）
- Commit：见 Notes「验收与提交」

## Notes

### 验收与提交

- 用户验收：2026-07-17 通过（A+B+C 合并验收；默认主题 monster、休息文案、⏸ 暂停托盘格式三项一并拍板）
- Commit：`8649916`（用户委托 AI 执行并推送）

### Gate 说明

Section A 无独立可见面（token/素材/工具），与 B、C 合并为一次用户验收——单独让用户验 CSS 变量没有意义。B、C 同屏（同一面板），也一并验收。

### 文案适配（待用户验收确认）

- 旧休息中 hint"切到任意网页,在锁屏上加时或等休息结束"指向已废弃的锁屏 → 改述为"起身休息一下;要继续就加时(扣配额)。"
- 底栏去掉 ⚙ / 🪟 emoji（DESIGN Don'ts:不用 emoji 当功能图标）；悬浮窗按钮删除（被菜单栏取代）

### 已拍板（2026-07-17）

- **默认主题 = monster**（用户实测后确认;store.rs 默认值已改,PRD §5 已标注）——与旧版默认隐藏小怪兽是有意偏差
- **休息中 hint 文案**定稿："起身休息一下;要继续就加时(扣配额)。"
- lastCategory 键名：旧 popup 写 `lastTaskCategory`，备份 schema 用 `lastCategory`——新版统一用 `lastCategory`，phase 06 导入时需映射旧键（已记入 phase 06 注意事项）

### 已知留白（按计划归后续 phase）

- 加时按钮：休息中可见但 disabled，加时选项流 phase 04
- 统计与设置按钮：disabled，窗口本体 phase 05
- 休息期面板副标题（MONSTER_SUBTITLES 随机句）：phase 04
