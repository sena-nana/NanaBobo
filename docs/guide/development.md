# 开发流程

## 架构总览

应用由两层组成,均为本仓库或仓库同级快照内的真实代码:

- `app/`:NanaUI L3 宿主 bin crate(crate 名 `nanabobo-app`,bin 名 `nanabobo`)。`app/src/main.rs` 实现 `RuntimeProgram`,以 `run_runtime` 启动 1200x800(最小 960x600)的原生窗口。壳层用 `DesktopShell` / 侧栏 / inspector / overlay,页面用 `AppContext::build` 与 `mount` 更新。业务命令直接调用 `nanabobo-core`,弹幕经 `EventSink` 进进程内消息队列。
- `crates/nanabobo-core/`:业务核心 crate。B站接口适配(`bilibili/`)、凭据存取(`credential_store`,Windows Keyring)、models、commands(AppState/AppError/ErrorCode)。零 UI 框架依赖。

界面作者层是 Rust L3,不是 Vue IIFE。导航用应用内 `Page` 枚举,不用 vue-router。

## 项目结构

`app/src/main.rs`:原生宿主入口与 `RuntimeProgram`。

`app/src/session.rs`:登录、房间、弹幕、快照会话与文件持久化。

`app/src/ui.rs`:DesktopShell 装配与各页 `mount`。

`crates/nanabobo-core/src/bilibili/`:B站第三方接口适配与字段映射。

`crates/nanabobo-core/src/credential_store.rs`:OS Keyring 与测试替身。

`crates/nanabobo-core/src/commands.rs`:业务命令、输入校验和错误映射。

`crates/nanabobo-core/src/events.rs`:`EventSink` 事件出口抽象。

## 本地前置

- Rust stable(≥ 1.92,`rust-toolchain.toml` 指向 stable)。
- `.nanaui-pin/`:仓库同级目录下的 NanaUI 构建快照,`nana-*` crate 以 path 依赖指向它,必须先就位。

## 构建与运行

`cargo build -p nanabobo-app`

`target/debug/nanabobo.exe`

## 测试

`cargo test`(根 workspace):nanabobo-core 业务测试 + app 会话单元测试。

## 边界

渲染器、原生壳与组件系统属于 NanaUI 上游(`.nanaui-pin/` 快照及其上游仓库),本仓库只记录上游问题,不修改快照;业务命令、凭据存储、L3 页面装配和 NanaBobo 专属契约属于本仓库。B站原始响应、请求头、Cookie 和 Token 不得越过 `nanabobo-core` 适配器边界;登录态只进入 Windows OS Keyring。
