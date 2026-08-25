# Tomato Monster · 按时停下来

一个 macOS 菜单栏番茄钟,给"时间到了却停不下来"的电脑工作者:**让"停下来"有提醒、有代价、有奖励**。

由同名 Chrome 插件迁移而来,产品哲学不变:

- 🍅 **按时停下来** —— 25 分钟到点,面板自动弹出并钉住(点不掉)、菜单栏变红、白噪音响起
- 💸 **加时有代价** —— 休息期间没有"跳过"按钮,想继续只能消耗每天 3 次的加时配额
- 🏅 **奖励"停下来"** —— 连续 7 天不用加时,love monster 全屏庆祝;年度热力图与连击记录
- 📤 **数据是你的** —— 本地优先(store.json),一键导出备份,每日任务和月度目标可手动同步到你自己的 Notion 数据库

## 安装

自用项目,不做签名与公证。首次打开需要绕过 Gatekeeper:

1. 下载 `.dmg`,把 `Tomato Monster.app` 拖进「应用程序」
2. **右键(或按住 ⌃ 点击)app → 打开 → 再点「打开」**——直接双击会被 macOS 拦下
3. 之后即可正常双击启动

要求 macOS 13+。

## 首次启动

第一次打开会有三步引导,按你的情况选:

**从没用过 Chrome 插件版?** 零配置开箱即用:

1. 引导第一步点「**我是新用户,直接开始**」
2. 按需选择是否开机自启、记住快捷键 `⌃⌥P`,引导结束
3. 点菜单栏的小怪兽图标 → 添加今天的任务 → 「开始专注」——就这些

数据从零开始积累,什么都不用配。Notion 同步是**可选**功能,想用的时候再去 统计与设置 → 📤 Notion 导出 填 Integration Token 和要使用的数据库 ID(不填完全不影响使用)。

**用过 Chrome 插件版?** 引导第一步点「导入备份文件」接上旧数据,备份怎么导出见下一节;错过引导也没关系,之后随时可在 统计与设置 → 💾 本地备份 里导入。

## 从 Chrome 插件迁移

1. 旧插件设置页 → 💾 本地备份 → 导出备份(得到 JSON 文件)
2. 新 app 首次启动会弹出引导,点「导入备份文件」选中它
   (或之后随时在 统计与设置 → 💾 本地备份 → 导入备份)
3. 历史、连击、热力图、徽章、设置全部接上;Notion 配置不进入备份,需在设置里重新填写 Integration Token 和数据库 ID
4. 停用/卸载 Chrome 插件

导入是整体覆盖且带校验:文件不对会整体拒绝,现有数据零改动。

备份 JSON 包含任务、目标和统计等个人历史,请妥善保管。App 在 macOS 上会把导出的文件权限设为仅当前用户可读写;复制到其他磁盘或云盘后,仍以目标位置的权限规则为准。

## 日常使用

- 点菜单栏小怪兽(或按 `⌃⌥P`)开始专注;快捷键可在设置里改
- 到点自动进入休息,休息结束默认自动开始下一轮(可关)
- 每完成 4 个番茄进入 20 分钟长休息(可调)
- 统计与设置:面板底部按钮打开,热力图 / 7 天图表 / 历史明细 / 备份 / Notion 都在里面

## Notion 同步（可选）

App 不连接任何中间服务器,Token 只保存在本机,请求只直连 Notion API。进入「统计与设置 → 📤 Notion 导出」可分别配置每日任务数据库和月度目标数据库:

- 「测试连接」检查既有每日任务导出。
- 「测试目标库」只读检查月度目标六个属性、时间框架 relation 和当前月唯一页面。
- 检查通过后,在「🎯 月度番茄目标」点击「同步本月到 Notion」。同步只处理 Rust 快照中的当前自然月;第一次创建,之后按「所属月 + 本地目标 ID」更新同一批记录。
- App 不做后台同步、历史月补同步或 Notion → App 回写;删除本地目标也不会删除 Notion 历史。

使用月度同步前,需将同一个 Integration 同时连接到月度目标库和「所属月」关联的时间框架库。Token 与全部数据库 ID 都不会进入备份。

每次月度同步都会显示新增 / 更新 / 失败计数。读取或 schema 预检失败时不会写入;单条写入失败可在修复权限或字段后直接重试。

## 开发

```bash
cargo tauri dev --config src-tauri/tauri.dev.conf.json
TOMATO_TEST=1 cargo tauri dev --config src-tauri/tauri.dev.conf.json
cargo test --manifest-path src-tauri/Cargo.toml
node --test src/stats/notion.test.mjs src/stats/goal-sync-ui.test.mjs
env -u TOMATO_TEST -u TOMATO_AUTOSTART -u TOMATO_OPEN_STATS cargo tauri build --target aarch64-apple-darwin --no-sign
```

开发命令必须合并 `tauri.dev.conf.json`：它将 App 显示为 **Tomato Monster Dev**，使用独立的 `com.tomatomonster.dev` 数据目录。普通开发模式从空数据开始；`TOMATO_TEST=1` 还会使用进程级临时 store。正式版固定安装并从 `/Applications/Tomato Monster.app` 启动，真实数据与 Notion 验收只在这份正式版中进行。

技术栈:Tauri 2(Rust 状态机 + vanilla JS 前端,无构建工具)。项目由 [First Flight](./AGENTS.md) 流程驱动开发,spec 文档见 BRIEF / PRD / DESIGN / ARCHITECTURE / CONTENT。

## License

[MIT](./LICENSE)
