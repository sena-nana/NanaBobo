# Nana播播工具箱

面向虚拟主播的 B 站直播桌面工具箱。基于 NanaUI L3（Rust `RuntimeProgram` + 原生控件），业务核心在 `crates/nanabobo-core`。

## 当前能力

- B站扫码登录、账号状态查询和退出登录。
- 直播间号查询：标题、主播、直播状态、在线人数和封面。
- 弹幕助手：只读展示当前房间弹幕与连接状态。
- 登录态保存在 Windows OS Keyring，不进入界面状态、本地文件存储或日志。

直播录制和回放整理会在稳定的事件与存储契约确定后逐步加入；当前不会显示未接通的入口。

## 开发

`cargo build -p nanabobo-app`

`target/debug/nanabobo.exe`

## 验证

`cargo test`（核心业务测试 + 应用单元测试）

更多约束见 [`AGENTS.md`](./AGENTS.md)、[`docs/guide/development.md`](./docs/guide/development.md) 和项目技能 `.agents/skills/nanabobo-development/SKILL.md`。
