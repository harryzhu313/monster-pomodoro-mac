# Phase 06 — 数据通道：备份迁移与 Notion 导出

**Status**: `completed`
**目标**: 旧插件备份 JSON 一次导入成功（校验 + 失败整体回滚），导出备份 schema v2，Notion 导出可用
**前置**: Phase 05（备份 / Notion 区 UI 骨架就位）；**需要用户提供一份旧插件导出的真实备份 JSON**

## 验收判据

- 用户提供的真实旧备份**一次导入成功**：历史、连击、热力图数据完整（对照旧插件显示抽查）
- 构造的坏文件（缺键 / 版本不对 / JSON 损坏）导入失败，**现有数据零改动**（整体回滚），失败文案含原因
- 导出的备份为 schema v2 且可被再次导入（兼容读 v1）
- Notion 导出到用户真实数据库成功；字段名与分类选项对齐 CONTENT.md 常量（产品红线 2：**不可改**）

## Sections

### Section A — 备份导入 / 导出

**Gate**: `committed`

**自测判据**：

- 用旧备份样本导入 → store.json 各键正确映射；浏览器特有 settings 项（悬浮窗等）被丢弃
- 三种坏文件分别导入 → 全部拒绝且 store.json 字节级不变
- 导出 → 再导入 → 数据一致（round-trip）

**Tasks**：

- [x] 导入：文件校验 → 整体校验 → 逐键映射 → 任一步失败整体回滚 (store.rs import_backup:副本上完成全部解析,全部成功才替换;v1 语义对齐:备份有的键覆盖、没有的回默认、timerState/quotaState 不动;lastTaskCategory→lastCategory 键名映射;单测 import_v1_backup_maps_all_keys / import_rejects_bad_files_without_touching_data 覆盖 5 种坏文件)
- [x] settings 里浏览器特有项导入时丢弃（ARCHITECTURE §4）(BROWSER_ONLY_SETTINGS=[lockscreenBg],解析前剔除;单测断言)
- [x] 导出：schema v2，保留对 v1 的读取兼容，保存路径用户选 (store.rs export_backup_json + lib.rs export_backup 走 tauri-plugin-dialog 保存对话框;导出不含 notionConfig——token 不外泄,单测断言;round-trip 单测)
- [x] 成功 / 失败反馈文案对齐 CONTENT.md"备份导入反馈"节 (stats.js:成功`导入完成,{N} 天的历史已接上...`/失败`导入失败,现有数据没有被改动。原因:{reason}...`;导入前确认弹窗平移旧文案,walk WKWebView 无原生 confirm → dialog 插件)

**记录**：

- 自测：2026-07-17 cargo test 26 passed(新增 3 项备份测试);带 dialog/http 插件启动无报错
- 用户验收：2026-07-17 通过（导出/坏文件回滚/真实旧备份迁移/Notion 测试与导出/透明度两轮调优）
- Commit：（提交后回填）

---

### Section B — Notion 导出

**Gate**: `committed`

**自测判据**：

- 配置区填入 Token / DB ID → "测试"按钮返回连接状态
- 历史明细里对某日执行"导入 Notion" → 该日任务出现在 Notion 数据库，字段逐一核对

**Tasks**：

- [x] `tauri-plugin-http` 接入，scope 仅 `https://api.notion.com/*`（ARCHITECTURE §6/§9）(capabilities/default.json http:default 带 url allowlist;插件注册 lib.rs)
- [x] 配置 UI 接线：Token / 任务 DB ID / 日页面 DB ID（可选）/ 测试按钮 / 状态提示 (stats.js:失焦/回车保存 → set_notion_config;测试按钮 → notion.testConnection,状态行三态)
- [x] 旧插件 Notion 导出逻辑近乎原样移植（fetch 换 plugin-http）(src/stats/notion.js 逐行移植旧 sw.js:902-1142:notionFetch/测试连接/日页面查询/分页查询/同名更新去重/导出汇总;历史明细"导入到 Notion"按钮接线,反馈弹窗文案平移)
- [x] 字段名 / 分类选项用 CONTENT.md 常量，代码里集中定义禁止散落 (notion.js 顶部 CATEGORY_VALUES + buildTaskPageProps 的字段名与旧版逐字一致,红线 2 注释标记)
- [x] Token 只存本地 store.json，确认不入 git、不出现在日志 (store.rs NotionConfig 独立字段,store.json 权限 600;导出备份不含 notionConfig;grep 确认无 token 日志输出;store.json 在 app data dir 天然不在仓库)

**记录**：

- 自测：2026-07-17 结构移植逐行核对;真实 Notion 连通性待用户验收(需要真实 token)
- 用户验收：2026-07-17 通过（导出/坏文件回滚/真实旧备份迁移/Notion 测试与导出/透明度两轮调优）
- Commit：（提交后回填）

## Notes

### 新增依赖登记

- `tauri-plugin-dialog`:备份导出保存对话框 + WKWebView 无原生 confirm/alert 的替代(ARCHITECTURE §4"保存对话框"的实现载体,已补登 §6 表格)
- `tauri-plugin-http`:ARCHITECTURE §6 原有规划,scope 锁 api.notion.com

### 验收中发现并修复的问题（2026-07-17）

1. **导出卡死（鼠标转圈）**：同步 command 在 Tauri 主线程执行,`blocking_save_file` 阻塞主线程,保存对话框依赖的事件循环无法响应 → 互等。修复:全部 22 个 command 改 `async fn`(调度到线程池),同时消除了"tick 持锁做窗口操作 × 主线程同步命令抢锁"的潜伏死锁窗口。
2. **统计窗口太透明看不清**（用户截图:亮色壁纸穿透）：vibrancy 材质 UnderWindowBackground → Sidebar,并补上 DESIGN.md 规定但先前漏做的"暖白/暖黑基调罩"(新 token --window-wash,深浅两套,盖在 vibrancy 上;主面板 Popover 材质本身够磨砂,不加罩)。

### 实现说明

- 导入入口用 HTML file input(WKWebView 原生支持,与旧版一致),文件内容读成字符串交 Rust 校验——不需要 fs 权限
- 旧版导入语义是"整体覆盖 7 键",新版一致;确认弹窗文案平移
- notionExportLog 的"已导入 N"绿标在 phase 05 已就位,本 phase 数据打通后自动点亮
