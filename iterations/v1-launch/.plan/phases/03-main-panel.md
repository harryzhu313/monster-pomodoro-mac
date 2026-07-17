# Phase 03 — 主面板

**Status**: `not started`
**目标**: 主面板 UI 完成日常操作闭环（计时 / 任务 / 收获 / 配额），PRD 场景 1"3 秒开始专注"成立
**前置**: Phase 02（状态机与 state-update 事件可用）

## 验收判据

- 点托盘 → 添加/选中任务 → 点"开始专注"，3 秒内进入专注（场景 1）
- 面板所有状态显示由 `state-update` 驱动，前端零自维护计时状态（AGENTS 规则 6）
- 文案对齐 CONTENT.md 主面板节（📦 平移旧 popup，不"优化"）
- theme.css 深浅两套 token 生效，深浅模式切换正常；除 theme.css 外 grep 不到硬编码 hex

## Sections

### Section A — 主题与素材基座

**Gate**: `pending`

**自测判据**：

- `cargo tauri dev` 面板加载 theme.css 无 404，深浅模式下背景 / 文字色正确切换
- `grep -rn '#[0-9a-fA-F]\{3,6\}' src/ --include='*.css' --include='*.js'` 仅命中 `shared/theme.css`
- `src/shared/monsters/` 有 5 只小怪兽 SVG

**Tasks**：

- [ ] 按 DESIGN.md YAML token 手写 `src/shared/theme.css`（深浅两套 CSS 变量）
- [ ] 从旧仓库复制素材：`themes/monster/*.svg` → `src/shared/monsters/`，`backgrounds/*.svg` → `src/shared/backgrounds/`
- [ ] `src/shared/util.js`：日期、mm:ss 格式化等共享工具

**记录**：

- 自测：
- 用户验收：
- Commit：

---

### Section B — 计时卡

**Gate**: `pending`

**自测判据**：

- `TOMATO_TEST=1` 走一轮：小怪兽表情、阶段文案、倒计时、主按钮全程与状态同步
- 放弃 → 记一颗烂番茄；重置 → 回 idle；配额显示"今日剩 N/3"

**Tasks**：

- [ ] 计时卡布局：小怪兽表情 / 阶段文案 / 倒计时 / 主按钮（开始·暂停）
- [ ] 放弃（记烂番茄，带确认提示 title）/ 重置
- [ ] 今日配额显示（剩 N/3）；加时按钮（仅休息中可见，具体加时流在 phase 04）
- [ ] 订阅 `state-update` 渲染、command 发指令的前端 IPC 封装

**记录**：

- 自测：
- 用户验收：
- Commit：

---

### Section C — 任务、收获与底栏

**Gate**: `pending`

**自测判据**：

- 添加任务（名称 / 分类四选一 / 计划番茄数）→ 列表出现；选中任务开始专注 → 完成后该任务计数 +1
- 空状态显示"还没有任务，先加一个吧。"，输入占位符"比如：内参阅读"
- 今日收获行：🍅 成熟 / 🥫 烂番茄 / 🔥 连击天数正确显示

**Tasks**：

- [ ] 任务添加表单（任务名 / 分类：工作·学习·生活·兴趣爱好 / 计划番茄数）
- [ ] 任务列表（当前专注标记、每任务完成计数、删除）
- [ ] 今日收获行（🍅 / 🥫 图标行 + 🔥 已连续 N 天）
- [ ] 底栏：打开统计与设置窗口按钮（窗口本体 phase 05）+ 随机激励提示语
- [ ] 任务数据走 Rust command 持久化（`tasksToday` 键）

**记录**：

- 自测：
- 用户验收：
- Commit：

## Notes

（执行中的临时记录写这里）
