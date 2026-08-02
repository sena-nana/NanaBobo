# Nana播播工具箱

面向虚拟主播的 B 站直播桌面工具箱。当前基于 Tauri 2、Vue 3、TypeScript 和 Rust，公共窗口、主题、设置及构建能力由 LiliaUI 提供，NanaBobo 只维护账号、直播工具和应用专属 Tauri 边界。

## 当前能力

- B站扫码登录、账号状态查询和退出登录。
- 直播间号查询：标题、主播、直播状态、在线人数和封面。
- 登录态保存在 Windows OS Keyring，不进入前端状态、普通设置或日志。

弹幕显示、直播录制和回放整理会在稳定的事件与存储契约确定后逐步加入；当前不会显示未接通的入口。

## 开发

`yarn install --immutable`

`yarn dev`

`yarn tauri:dev`

## 验证

`yarn agent:debug --json`

`yarn test`

`yarn build`

`cargo check --manifest-path src-tauri/Cargo.toml`

`yarn verify`

更多约束见 [`AGENTS.md`](./AGENTS.md)、[`docs/guide/development.md`](./docs/guide/development.md) 和项目技能 `.agents/skills/nanabobo-development/SKILL.md`。
