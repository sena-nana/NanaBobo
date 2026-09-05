# Nana播播工具箱

面向虚拟主播的 B 站直播桌面工具箱。基于 NanaUI L3（Rust `RuntimeProgram` + 原生控件），业务核心在 `crates/nanabobo-core`。

## 当前能力

- 概览启动台：查看当前主播、直播状态、人气、粉丝数与近期趋势，并启动桌面弹幕。
- 独立桌面弹幕：透明、置顶的纵向消息流；调整大小和字号后可锁定穿透，从主界面解锁。关闭窗口停止接收，当前房间的内存历史保留。
- B 站扫码登录、账号状态查询和退出登录；关闭或刷新二维码会取消旧登录流程。
- 房间统计与历史：按房间保留采样时间和数据，缺失指标保持缺失，支持清除所选房间历史。
- 外观设置：主题、窗口材质与背景透明度保存在用户目录，原生材质的可用效果取决于 Windows 环境。
- 登录态仅保存在 Windows OS Keyring；弹幕内容只留在内存，不写入文件或日志。

非凭据数据保存在 `%LOCALAPPDATA%/NanaBobo/state.json`。首次运行读取旧版可执行文件旁的 `nanabobo-storage.json` 并迁移，保留旧文件；发现损坏数据时停止覆盖，明确重试后先备份再恢复保存。

直播录制与回放整理尚未实现，不提供入口。

## 开发

`cargo build -p nanabobo-app`

`target/debug/nanabobo.exe`

## 验证

`cargo test`（核心业务与会话、存储、资源生命周期测试）

`cargo clippy -p nanabobo-app -p nanabobo-core --all-targets --no-deps -- -D warnings`

`cargo test -p nanabobo-app ui::acceptance -- --ignored --test-threads=1`（真实 GPU 界面与交互验收，截图输出到 `target/ui-acceptance/`）

自动化测试不等同于已验证真实扫码登录、B 站网络连接或 Windows 材质显示；发布前还需在本机完成这些手动冒烟检查。

更多约束见 [`AGENTS.md`](./AGENTS.md)、[`docs/guide/development.md`](./docs/guide/development.md) 和项目技能 `.agents/skills/nanabobo-development/SKILL.md`。
