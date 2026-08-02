# 开发流程

## 项目结构

`src/contracts/`：前端可见的稳定业务契约。

`src/features/account/`：扫码登录、账号状态、退出登录。

`src/features/live/`：房间信息查询。

`src/features/home/`：真实可用的首页组合。

`src/services/`：Tauri invoke 与运行时边界。

`src/ui/`：LiliaUI 应用 facade 和 Shell 装配。

`src-tauri/src/bilibili/`：B站第三方接口适配与字段映射。

`src-tauri/src/credential_store.rs`：OS Keyring 与测试替身。

`src-tauri/src/commands.rs`：Tauri 命令、输入校验和错误映射。

`src-tauri/src/models.rs`：跨端输出模型。

## 本地运行

仓库统一使用 Node.js 26、Yarn 4 和 Rust toolchain。首次准备环境后运行：

`yarn install --immutable`

`yarn dev`

`yarn tauri:dev`

## 验证

`yarn agent:debug --json`

`yarn test`

`yarn build`

`cargo check --manifest-path src-tauri/Cargo.toml`

`yarn verify`

若只修改项目技能，运行 `quick_validate.py .agents/skills/nanabobo-development`。

## LiliaUI 边界

共享 Shell、主题、设置、窗口状态、构建包装器和公共组件属于 LiliaUI；业务页面、路由、应用命令、B站适配器、凭据存储和 NanaBobo 专属契约属于本仓库。不要直接编辑 `node_modules/@lilia/*`，也不要在业务页面中直接导入具体 UI Layer。
