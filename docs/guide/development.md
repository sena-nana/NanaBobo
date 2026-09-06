# 开发流程

## 架构与责任

应用采用三个责任层，界面仅使用 Rust NanaUI L3，不引入 WebView、Vue 或 Node 产品界面。

- `app/` 是原生宿主。`main.rs` 按 WindowId 路由主窗口和桌面弹幕窗口，各自拥有文档与视图，共享 Session 和纹理上下文；`ui.rs` 装配概览、统计、设置，桌面层独立装配。
- `app/src/session/` 按认证、当前房间、弹幕、统计、资源与存储分离状态和操作。异步完成携带请求身份；取消、换房、退出后拒绝旧结果。`Session::advance(now)` 消费独立计时器，每个流程只允许一个正在执行的请求。
- `crates/nanabobo-core/` 负责 B 站适配、业务模型、稳定错误与 Keyring。认证提交和退出在同一边界串行处理；弹幕状态由连接管理器维护，服务端鉴权成功后才发布已连接。
- NanaUI 上游负责通用组件、滚动、图表、窗口、场景和渲染。业务行为不进入上游；上游缺陷在 NanaUI 源仓库修复。

Core 通过 `CoreEvent` / `EventSink` 发送类型化事件；旧的 `nanabobo://danmaku/message` 与 `nanabobo://danmaku/status` 命名适配仍可用。宿主控制事件与有界弹幕队列分离，批量消费弹幕，避免消息积压阻塞用户操作。

NanaUI 在应用输入钩子前通过 `RuntimeInputAdapter` 分发原生输入；旧 Vue 委托只用于 JS 桥接，不应重复分发。无障碍动作沿用 `RuntimeProgram::accessibility_action`，通过 `with_document_mut` 按窗口路由。业务回调统一经 `Inbox → Wake → update` 更新；应用快捷键须尊重 `InputDisposition::prevent_default`。

弹窗关闭监听挂在 `OverlayHost`，按 `OverlayClosing.root` 区分登录与清理确认。Session 主动关闭时先停用该监听，再关闭原生浮层，避免产生影响下一次打开的重复关闭事件。

统计刷新保留现有 Tabs 模型及实体。上游 `crates/nana-ui-runtime/src/tabs.rs` 尚未通过 `ComponentView::reconcile` 保留 `option_nodes`，重新传入 `Tabs::new` 会使 `framework/selection.rs::sync_tabs_options` 重建选项。复现：ArrowRight 切到历史后，应用更新清空焦点，ArrowLeft 无效。应用保留原控件模型避免此问题；通用协调行为的修复属于 NanaUI 上游。

## 概览、桌面弹幕与数据

概览展示当前房间指标、最近 30 次采样趋势与桌面弹幕启动入口；统计页可查看所选房间完整历史。选房不启动弹幕，独立窗口创建成功才开始连接。窗口锁定在系统确认鼠标穿透成功后生效；主界面可以解锁或关闭。关闭桌面层停止连接并保留内存历史，关闭主窗口退出应用。房间输入与当前房间分别保存，失败的换房请求保留当前上下文。未实现的录制、回放和发送弹幕不提供操作入口。

`%LOCALAPPDATA%/NanaBobo/state.json` 保存版本、记住的房间、统计历史和全部外观设置（主题、窗口材质、透明度、透明区域、标题栏跟随、工作区圆角）。后台线程合并保存请求，通过同目录临时文件替换；正常退出等待最后一次保存完成。首次启动兼容旧版可执行文件旁的 `nanabobo-storage.json`，迁移保留原件。损坏或不支持的数据不自动覆盖，用户重试时先备份原文件。凭据只进入 Windows OS Keyring；弹幕文本仅留在内存。

图片下载有字节上限；解码前检查尺寸、像素数量与解码预算。槽位切换使旧下载失效，移除槽位同时释放注册纹理、GPU 引用和解码缓存。

## 本地依赖与运行

安装 Rust stable，按 `rust-toolchain.toml` 使用工具链。NanaUI path 依赖指向仓库同级的 `.nanaui-pin/`；当前开发环境中该目录是指向同级 `NanaUI` 的 Junction，修改上游即影响依赖，无需复制同步。其他环境应先检查链接与路径，不能假设一定是独立快照。

```powershell
cargo build -p nanabobo-app
./target/debug/nanabobo.exe
```

默认窗口 1200×800，最小 960×600。

桌面弹幕默认 420×640、最小 280×240，独立置顶且启动不抢焦点。保存逻辑位置、尺寸、字号和背景透明度；重启不自动启动，重开进入调整模式。窗口恢复按当前显示器工作区校正。

## 验证

```powershell
cargo test
cargo clippy -p nanabobo-core --all-targets -- -D warnings
cargo build -p nanabobo-app
```

功能测试覆盖认证取消与凭据提交、连接状态和协议应答、解析预算、异步旧结果、消息队列、图片身份、统计投影与存储恢复。定向存储验证可用 `cargo test -p nanabobo-app session::storage`。

改动跨窗口或交互时，构建成功后手动检查：真实扫码与取消、退出不恢复旧登录、连接/换房/断房、弹幕断线与跟随、无数据与错误恢复、历史清除、设置重启恢复、浅深色及 Windows 原生材质。自动化检查不能证明第三方服务当前可用，最终报告应区分已运行的测试和实际完成的冒烟项目。

## 边界

B 站原始响应、Cookie、请求头和 Token 不进入 UI 模型、文件存储或日志。界面只展示安全业务错误；任何可见操作都应完成真实行为或表达真实不可用状态。测试使用内存凭据替身，不调用真实账号或保存真实凭据。
