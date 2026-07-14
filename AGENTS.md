# Tomato Monster (macOS) · AGENTS

> 这份文档是 AI 进入本项目的入口。请先读完这份文档再开始工作。

## 项目一句话

一个 macOS 菜单栏番茄钟应用,给"时间到了却停不下来"的电脑工作者,让"停下来"有提醒、有代价、有奖励。

## 文档地图

本项目由 First Flight 生成。**长期文档先读,迭代产物按需读**:

**长期文档(项目根,跨迭代共用):**

| 文档 | 内容 |
|---|---|
| [BRIEF.md](./BRIEF.md) | 项目长期纲领(本质、用户、价值、边界) |
| [DESIGN.md](./DESIGN.md) | 视觉与 UX 风格(Google DESIGN.md 标准,YAML token 是权威) |
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Tauri 2 技术决策、Rust/前端职责划分、代码组织 |
| [AGENTS.md](./AGENTS.md) | 本文件——AI 入口 + 协作规则 |

**迭代产物(`iterations/v{N}-{slug}/`):**

| 文件 | 内容 |
|---|---|
| `iterations/v1-launch/PRD.md` | v1 功能、六个交互面、信息架构、目标衡量 |
| `iterations/v1-launch/CONTENT.md` | 内容槽索引(📝 直接文案 / 🤖 生成需求 / 📦 数据源) |
| `iterations/v{N}-{slug}/.plan/` | 各迭代的 plan + phase 实施目录 |

**特殊资产**:旧 Chrome 插件仓库 `/Users/apple/web-projects/tomato-timer-v2` 是**行为与 UI 的参考基准**——本项目 v1 的本质是把它迁移为菜单栏 app,功能对齐为先。

## 项目本质边界(来自 BRIEF)

**永远会是的**:
- macOS 菜单栏常驻应用
- 本地优先(数据在本地与用户自己的 Notion)
- "强制停下来"的休息哲学

**永远不做的**:
- 跳过休息的出口(任何形态)
- Windows / Linux 版
- 为增长与分发的额外投入(上架推广、账号体系)

⚠️ 任何看似偏离这些边界的请求,请先停下来和用户确认,不要默认接受。

## 产品红线(违反 = bug)

1. **休息期间没有免费跳过出口**——只有"加时(扣配额)"和"等待"两条路;快捷键、托盘菜单、任何入口都不得绕过。
2. **分类选项(工作/学习/生活/兴趣爱好)与 Notion 字段名不可改**——用户既有 Notion 数据库依赖这些常量,见 CONTENT.md"全局"节。
3. **奖励"停下来",不奖励"番茄数"**——任何激励/徽章/动效设计不得反向强化"不想停"的倾向。

## 技术栈一句话(来自 ARCHITECTURE)

Tauri 2.x(macOS 13+):Rust 负责状态机/计时/持久化/音频/托盘,前端 vanilla JS + CSS 变量、无构建工具、纯视图。详细决策见 [ARCHITECTURE.md](./ARCHITECTURE.md)。

## 写代码前的准备

如果还没做完这些,先停下来提醒用户:

- [ ] Rust 工具链(rustup)+ Xcode Command Line Tools + `cargo install tauri-cli`
- [ ] 从旧仓库复制素材:`themes/monster/*.svg` → `src/shared/monsters/`,`backgrounds/*.svg` → `src/shared/backgrounds/`
- [ ] 按 DESIGN.md 的 YAML token 手写 `src/shared/theme.css`(深浅两套 CSS 变量,禁止硬编码 hex 散落)

## 写代码时的核心规则

### 必须遵循

1. **行为存疑,回旧仓库查证**——移植时任何交互、计时、结算细节不确定,先读旧插件代码(popup / options / lockscreen / service-worker)确认行为,禁止凭印象发挥(ARCHITECTURE §8)。
2. **遵循 DESIGN.md 的 Do's & Don'ts**——尤其 Don'ts:不加载外部字体、不用彩色菜单栏图标、不渐变大色块、不用 emoji 当功能图标、颜色只经 CSS 变量 token。
3. **CONTENT.md 三种模式**:📝 原样使用,不要替换或"优化";🤖 按需求 + 品牌语气生成,生成完主动给用户看;📦 从指定路径(多为旧仓库)平移。
4. **不引入 ARCHITECTURE 未列出的依赖**——需要新 crate / Tauri 插件,先停下来讨论、更新 ARCHITECTURE 再装。
5. **复杂开发用 phase 管理**——改动 ≥ 5 步骤 / 跨多文件 / 跨多次会话时,必须建 `iterations/v{N}-{slug}/.plan/plan.md` + `phases/NN-*.md`,每个 phase 完成后停下让用户验收,**不要一口气做完**。
6. **Rust 是唯一事实来源**——前端不得自己维护计时状态,只订阅 `state-update` 事件、发 command;所有状态变更走 Rust。

### 编码风格

- Rust:标准 rustfmt;模块划分见 ARCHITECTURE §7
- 前端:vanilla ES modules,无构建;复杂数据结构用 JSDoc 注释
- 注释密度与语言跟随旧仓库风格(中文注释为主)

### 提交前自检

- [ ] `TOMATO_TEST=1 cargo tauri dev` 跑通一轮完整番茄周期(专注 → 提醒 → 休息 → 下一轮)
- [ ] 发布向提交确认 TEST_MODE 关闭
- [ ] 改了 DESIGN.md 则跑 `npx @google/design.md lint DESIGN.md`

## Spec Sync(文档与代码同步)

- **改动影响长期文档** → 增量更新根目录文件(换 crate → ARCHITECTURE;调色 → DESIGN)
- **改动影响当前迭代** → 更新 `iterations/v1-launch/` 下文件
- **新需求 / 新功能** → 开新迭代 `iterations/v{N+1}-{slug}/`,不回改 v1 的 PRD
- **与 BRIEF"永远不做"冲突** → 停下,这是 BRIEF 级变更,要么用户重审 BRIEF 要么收窄需求
- 判断原则:"下次 AI 看到这个项目时需要知道吗?"是 → 进对应 spec;否 → 实现层微调,不必。

## 工具链常用命令

```bash
cargo tauri dev                 # 开发运行
TOMATO_TEST=1 cargo tauri dev   # 测试模式:专注 15s / 休息 30s(单一开关,只在 Rust 侧)
cargo tauri build               # 构建 .app / .dmg(自用不签名)
npx @google/design.md lint DESIGN.md   # 改 DESIGN 后校验 token 与对比度
```

## 在哪里找信息

| 问题 | 去哪查 |
|---|---|
| 某交互旧版是怎么做的? | 旧仓库对应文件(popup / options / lockscreen / service-worker) |
| 这个项目长期要变成什么样? | BRIEF.md |
| v1 要做哪些界面 / 功能? | iterations/v1-launch/PRD.md §4-5 |
| 颜色 / 字体 / 圆角用什么? | DESIGN.md YAML front matter |
| 这个面板放什么内容 / 文案? | CONTENT.md 对应槽 |
| 存储 schema / 备份导入格式? | ARCHITECTURE.md §4 |
| 窗口 / 托盘 / 插件选型? | ARCHITECTURE.md §3、§6 |
| 性能 / a11y 预算? | ARCHITECTURE.md §9 + DESIGN.md §Accessibility |

## 协作姿态

- 不确定先问,不要默认假设
- 遇到 BRIEF 边界或产品红线冲突要停下
- 改了行为要回头更新对应 spec——5 份文档是 source of truth,代码是它们的实现

## 这是用 First Flight 生成的

本项目的 6 份文档(BRIEF / PRD / DESIGN / ARCHITECTURE / CONTENT / AGENTS)由 First Flight skill 系列引导生成,生成于 2026-07-14。
