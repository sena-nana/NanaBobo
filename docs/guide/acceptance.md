# 双层验收矩阵

CI 对远端固定依赖和本地 LiliaUI 联调依赖分别执行边界检查、测试、生产构建和 Tauri 无安装包编译：

| 依赖来源 | 预期 Layer |
| --- | --- |
| 固定 Git commit | `@lilia/ui` |
| 本地 portal | `@lilia/ui` |

## 本地完整检查

```bash
yarn agent:debug --json
yarn verify
yarn tauri:build:no-bundle
```

构建会生成 `dist/ui-bundle-report.json`，并与 `tests/bundle-baseline.json` 比较。报告必须证明单 Layer、异步入口数量和体积预算同时满足。
