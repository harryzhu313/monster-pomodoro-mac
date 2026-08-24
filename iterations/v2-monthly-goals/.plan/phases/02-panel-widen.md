# Phase 02 — 面板加宽与两行任务行

**Status**: `not started`
**目标**: 面板 360×560，任务行改两行排，10–22 字的任务名不再挤压换行
**前置**: Phase 01（避免和数据修复混在一次提交里）

## 验收判据

- 面板宽 360px、高 560px；托盘点击弹出位置仍正确（多显示器下 positioner 不回退到右上角）
- 任务行两行排：第一行 复选框 + 任务名 + 分类徽章 + 删除；第二行 番茄进度 + 「设为当前」
- 任务名「完成网站建设的首页设计稿」（12 字）在第一行内不换行
- 添加框的 placeholder「比如：内参阅读」完整可见，不被截断
- 空状态、首启引导层、休息期加时区在新宽度下布局不塌

## Sections

### Section A — 窗口尺寸与任务行重排

**Gate**: `pending`

**自测判据**：

- `cargo tauri dev` 目视对照设计稿「★ 定稿方案」板
- 任务数 0 / 1 / 6 三种情况下面板不出现纵向滚动条（6 条以上任务列表内部滚动）

**Tasks**：

- [ ] `tauri.conf.json` panel 窗口 width 300→360、height 480→560
- [ ] `panel.css` 任务行改两行布局（`.task-item` 两行版），番茄进度与「设为当前」沉到第二行
- [ ] `panel.js` `renderTasks` 按新 DOM 结构生成（保持 `lastTasksJson` 差量重建逻辑不变）
- [ ] 回归首启引导层 / 休息期加时区 / 空状态在 360px 下的布局

**记录**：

- 自测：
- 用户验收：
- Commit：

---

## Notes

- 颜色一律走 theme.css token，不新增硬编码 hex（AGENTS 编码规则 2）。
- DESIGN.md 里若记录了面板尺寸，phase 06 统一同步，本 phase 不改 spec。
