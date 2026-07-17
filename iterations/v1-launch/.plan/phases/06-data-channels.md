# Phase 06 — 数据通道：备份迁移与 Notion 导出

**Status**: `not started`
**目标**: 旧插件备份 JSON 一次导入成功（校验 + 失败整体回滚），导出备份 schema v2，Notion 导出可用
**前置**: Phase 05（备份 / Notion 区 UI 骨架就位）；**需要用户提供一份旧插件导出的真实备份 JSON**

## 验收判据

- 用户提供的真实旧备份**一次导入成功**：历史、连击、热力图数据完整（对照旧插件显示抽查）
- 构造的坏文件（缺键 / 版本不对 / JSON 损坏）导入失败，**现有数据零改动**（整体回滚），失败文案含原因
- 导出的备份为 schema v2 且可被再次导入（兼容读 v1）
- Notion 导出到用户真实数据库成功；字段名与分类选项对齐 CONTENT.md 常量（产品红线 2：**不可改**）

## Sections

### Section A — 备份导入 / 导出

**Gate**: `pending`

**自测判据**：

- 用旧备份样本导入 → store.json 各键正确映射；浏览器特有 settings 项（悬浮窗等）被丢弃
- 三种坏文件分别导入 → 全部拒绝且 store.json 字节级不变
- 导出 → 再导入 → 数据一致（round-trip）

**Tasks**：

- [ ] 导入：文件对话框 → 整体校验（`BACKUP_SCHEMA_VERSION: 1`，七键齐全）→ 逐键映射 → 任一步失败整体回滚
- [ ] settings 里浏览器特有项导入时丢弃（ARCHITECTURE §4）
- [ ] 导出：同 schema 版本号 +1，保留对 v1 的读取兼容，保存路径用户选
- [ ] 成功 / 失败反馈文案对齐 CONTENT.md"备份导入反馈"节

**记录**：

- 自测：
- 用户验收：
- Commit：

---

### Section B — Notion 导出

**Gate**: `pending`

**自测判据**：

- 配置区填入 Token / DB ID → "测试"按钮返回连接状态
- 历史明细里对某日执行"导入 Notion" → 该日任务出现在 Notion 数据库，字段逐一核对

**Tasks**：

- [ ] `tauri-plugin-http` 接入，scope 仅 `https://api.notion.com/*`（ARCHITECTURE §6/§9）
- [ ] 配置 UI 接线：Token / 任务 DB ID / 日页面 DB ID（可选）/ 测试按钮 / 状态提示
- [ ] 旧插件 Notion 导出逻辑近乎原样移植（fetch 换 plugin-http）
- [ ] 字段名 / 分类选项用 CONTENT.md 常量，代码里集中定义禁止散落
- [ ] Token 只存本地 store.json，确认不入 git、不出现在日志

**记录**：

- 自测：
- 用户验收：
- Commit：

## Notes

（执行中的临时记录写这里）
