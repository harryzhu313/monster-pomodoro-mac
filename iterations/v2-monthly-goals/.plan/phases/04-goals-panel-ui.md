# Phase 04 — 面板月度目标 chips 与今日番茄数

**Status**: `in progress`（用户已验收，待提交）
**目标**: 面板上勾选 chip 即加入今日任务；今日番茄数可在任务行直接改
**前置**: Phase 03（数据与 commands 就位）

## 验收判据

- 「今日任务」卡顶部渲染月度目标行：小标题 +「管理 ›」+ chips + 合计进度条
- chip 显示 `目标名 已完成/本月总量`；勾选态填番茄红；已投入锁定态不可点且有 title 提示
- 点 chip → 任务按预设名称与分类进入今日任务，番茄数默认 1
- 任务行第二行的 `type=number` 数字框可改今日番茄数，**不能小于今天已投入的数量**
- 改番茄数不回写月度目标；取消勾选再勾回来，番茄数回到默认 1
- 目标为空时月度目标行整体不渲染（不占高度），面板回到 phase 02 的样子

## Sections

### Section A — chips 行

**Gate**: `user-approved`

**自测判据**：勾选 / 取消 / 锁定三态实测；目标 0 个与 5 个时面板高度可接受

**Tasks**：

- [x] `panel.html` 「今日任务」卡顶部加 `.goal-strip` 结构
- [x] `panel.css` chips 与进度条样式；勾选态颜色新增 `--goal-chip-on-bg` / `--goal-chip-on-border` 两个 token（深浅各一套），未硬编码 hex
- [x] `panel.js` `renderGoals()` 差量重建（`lastGoalsJson`，避免每秒快照打断交互）+ 绑定 `toggle_goal_today`
- [x] 锁定态 chip 设 `disabled` 且带解释性 title，不只是视觉变灰
- [x] 月份文案取自快照的 `today`，不读本地时钟
- [x] 「管理 ›」复用 `open_stats`
- [x] 目标为空时 `.goal-strip` 整块隐藏

### Section B — 今日番茄数可改

**Gate**: `user-approved`

**自测判据**：敲数字 / 点上下箭头 / 敲小于已投入的数 三种操作行为正确

**Tasks**：

- [x] 任务行第二行加 `type=number` 数字框（沿用「添加」那行的视觉，自带上下箭头）
- [x] `set_task_planned` command —— **已在 phase 03 前移完成**
- [x] 提交时机：回车 / 失焦；聚焦全选；Esc 取消回弹
- [x] `min` 跟随今天已投入数，已完成的任务输入框 `disabled`
- [x] 计划数 > 8 时番茄图标改数值显示（`🍅 3/30`）
- [x] **方案回退**：移除 `ResizeObserver → resize_panel` 动态调窗链路，主面板固定为 360×700px（`src-tauri/tauri.conf.json:17-18`；验证命令：`rg -n 'resize_panel|ResizeObserver|syncPanelHeight' src src-tauri/src` 无匹配）
- [x] **修掉一个实测发现的 bug**：清空输入框时 `Number('')` === 0 是合法有限数，会把 `planned: 0` 发给后端；改为空串 / < 1 / 非数字一律回弹，不劳烦后端

**记录**：

- 自测（2026-08-24，`panel.js` 浏览器降级模式 + mock 月度目标数据）：
  - chips 渲染正确：勾选态 / 锁定态（`disabled`）/ 进度 `12/30` / 合计进度条 39%
  - 数字框 `min` 跟随已投入数（used=2 的任务 min=2），已完成任务的输入框 disabled
  - 五种输入场景行为正确：改成 7 → 发出 `set_task_planned`；清空 / 填 0 / 填与原值相同 / Esc → 一律回弹且不发指令
  - 点锁定 chip 不发指令；点未勾选 chip 发出 `toggle_goal_today`；「管理 ›」发出 `open_stats`
  - 360px 下零横向溢出；3 个目标 chips 折两行
  - 固定 700px 方案回退：JavaScript module 语法与 Tauri JSON 校验通过；`cargo check` 通过；`cargo test` 40 项全绿；开发版重新启动供用户真机验收
- 用户验收：2026-08-24，主面板固定 360×700px 的真机高度通过（用户反馈：“高度可以”）；随后用户确认 Phase 04 整体验收，Section A / B 均通过
- Commit：

---

## Notes

- 前端零计算：chip 的进度、锁定态全部读快照字段，不在 JS 里算（AGENTS 规则 6）。
- `set_task_planned` 的下限夹紧必须在 Rust 侧做，前端的限制只是体验优化。

### ⚠️ 实测发现:面板内容高度早已超过窗口高度(非本 phase 引入)

在 360px 宽下实测 `.container` 高度：

| 场景 | 内容高 | 相对 560px 窗口 |
|---|---|---|
| v1 基线：无目标 + 1 任务 | 648px | 超 88px |
| 无目标 + 3 任务 | 791px | 超 231px |
| 3 目标 + 0 任务 | 676px | 超 116px |
| 3 目标 + 1 任务 | 744px | 超 184px |
| 3 目标 + 3 任务 | 887px | 超 327px |

去掉月度目标区后仍然超高 → **v1 时期就存在**（当时窗口 480px，超得更多），phase 02 把窗口 480→560 反而缓解了 80px。任务列表自身 `max-height: 220px` 内部滚动，所以总高有天花板（约 890px）。`body` 的 `overflow-y: auto`，即面板一直是可滚动的。

固定高度的大头是计时卡 301px（小怪兽 82 + 48px 计时 + 两排按钮 + 配额）。

**最终决策（用户 2026-08-24 复看动态效果后确认）：主面板固定为 700px。**

先按用户要求把窗口提到 700px，实测仍不够——"3 目标 + 1 任务"744px 超 44px，而"3 目标 + 3 任务"887px 超 187px；反过来最空态 580px 又会留白 120px。期间曾试过内容自适应，用户真实数据（3 目标 + 2 任务）会把面板撑到约 809px；复看真机效果后，用户认为动态高度视觉占用过大，决定接受固定高度下的留白与滚动，恢复 700px。

最终方案：

- `tauri.conf.json` 是主面板尺寸的唯一事实来源：宽 360px、高 700px
- 前端不再观察或上报内容高度，Rust 不再提供运行时调高面板的 command
- 内容超过 700px 时保持既有纵向滚动；任务列表仍以 220px 为上限并在列表内部滚动

动态方案的真机试验记录保留为决策依据：用户真实数据曾触发 809px，浏览器预测 815px，相差 6px；相关运行时代码现已移除。
