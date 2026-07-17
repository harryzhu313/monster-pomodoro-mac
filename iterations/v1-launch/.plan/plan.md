# 把 Chrome 插件版番茄钟迁移为 macOS 菜单栏 app，作者日常番茄完全脱离浏览器

> 文件位置：`iterations/v1-launch/.plan/plan.md`
> 配套 skill：first-flight-phases
> 本 plan 文档是**稳定航图**，状态跟踪在各 phase 文档（同目录的 `phases/NN-*.md`）里。

## 背景

6 份 spec（BRIEF / PRD / DESIGN / ARCHITECTURE / CONTENT / AGENTS）已定稿，仓库已公开（github.com/harryzhu313/monster-pomodoro-mac）。v1 的本质是**功能对齐迁移**：把旧 Chrome 插件（`../tomato-timer-v2`，本地私有仓库）的全部能力平移到 Tauri 2 菜单栏应用，新增菜单栏形态必要能力（托盘倒计时、全局快捷键、开机自启），并通过备份 JSON 一次性迁移历史数据。上线标准：作者的日常番茄在新 app 里闭环，插件停用。

## 范围

**做：**

- 核心计时（25/5 + 长休息）、托盘图标与倒计时
- 主面板（计时 / 任务 / 收获 / 配额）
- 统计与设置窗口（连击 / 热力图 / 7 天 / 历史 / 备份 / Notion / 全部设置）
- 休息强提醒 + 白噪音 / 提示音、小怪兽情绪主题
- 全局快捷键、开机自启、首次启动引导
- 插件备份 JSON 导入（校验 + 整体回滚）

**不做：**

- 强制锁屏（v2 重新评估，候选"缓冲式锁屏"）
- "结束要说出口"（元认知输入）、日末自动提醒
- 手机端 / 手机同步、桌面悬浮窗
- 签名公证、上架分发、账号体系、Windows / Linux

## 阶段总览

| #  | 阶段 slug          | 一句话目标                                                       | 状态        |
|----|--------------------|------------------------------------------------------------------|-------------|
| 01 | scaffold-spikes    | Tauri 骨架跑起来 + 4 项技术风险 spike 得出 go/no-go               | completed   |
| 02 | timer-core         | Rust 状态机 + 持久化，行为对齐旧 service-worker，TEST 全周期跑通  | completed   |
| 03 | main-panel         | 主面板 UI（计时 / 任务 / 收获 / 配额），日常操作闭环              | completed   |
| 04 | break-intervention | 到点干预：chime + 白噪音 + 钉住面板 + 托盘变红 + 加时扣配额      | completed   |
| 05 | stats-settings     | 统计与设置窗口（连击 / 热力图 / 7 天 / 历史 + 全部设置项）        | completed   |
| 06 | data-channels      | 旧备份导入（校验 + 回滚）+ 导出 + Notion 导出                     | in progress |
| 07 | system-polish      | 全局快捷键、自启、首启引导、庆祝弹层、性能验收、build .dmg        | not started |

> 状态值：`not started` / `in progress` / `completed` / `blocked` / `skipped`
>
> 详细任务、evidence、blocker 在各 phase 文档（`phases/NN-<slug>.md`）里，**不在本表里展开**。

## 关键决策

- **2026-07-17**：phase 01 只做 spike 不做功能（ARCHITECTURE §11 硬性要求）——vibrancy 兼容性、托盘秒刷功耗、rodio 音色、positioner 多显示器四项有一票否决权，先验证再投入。
- **2026-07-17**：Rust 先行（02）再做 UI（03）——ARCHITECTURE 定了"Rust 是唯一事实来源，前端纯视图"，状态机没立住之前写 UI 只会返工。
- **2026-07-17**：04 结束即达到"每天可用"——计时 + 面板 + 干预构成最小日常闭环，从 05 起作者可以边 dogfood 边开发，用真实使用暴露问题。
- **2026-07-17**：备份迁移和 Notion 单独成 phase（06）——迁移一次成功是上线硬标准，校验 + 整体回滚逻辑值得独立验收，不和 UI 开发混在一起。
- **2026-07-17**：**系统通知全删，改为休息时面板自动弹出并钉住**（用户在 phase 04 验收中提出）——通知是"可忽略的提醒"，钉住面板介于提醒与锁屏之间，更贴合"强制停下来"哲学且不需要通知权限；PRD/CONTENT/ARCHITECTURE 同步更新，phase 07 首启引导随之减一步。

## Open Questions

- [x] 托盘每秒刷新功耗是否超预算（CPU < 0.5%）— phase 01 实测 ~0.1%，保留秒级刷新
- [x] rodio 合成白噪音音色能否接受 — phase 01 对齐旧版棕噪音算法后用户验收通过（详见 phase 01 Notes）
- [x] window-vibrancy 与 Tauri 2.x 当前版本兼容性 — phase 01 验证通过（tauri 2.11.5 + vibrancy 0.6）
- [ ] 默认快捷键 ⌃⌥P 是否与常用软件冲突 — 预期在 phase 07 解决（可自定义兜底）
- [ ] 需要一份**旧插件导出的真实备份 JSON** 作为 phase 06 的导入测试样本 — 需要用户从旧插件 options 页导出

## 关联

> 路径相对当前文件位置 `iterations/v1-launch/.plan/plan.md`：

- 长期文档（项目根）：[BRIEF.md](../../../BRIEF.md) / [DESIGN.md](../../../DESIGN.md) / [ARCHITECTURE.md](../../../ARCHITECTURE.md) / [AGENTS.md](../../../AGENTS.md)
- 当前迭代 PRD：[PRD.md](../PRD.md)
- 首版 CONTENT：[CONTENT.md](../CONTENT.md)
