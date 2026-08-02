# 桌面应用脚手架

本仓库是最小 Tauri + Vue 应用脚手架。公共 UI、配置、工具、构建流程和 Tauri 窗口状态插件来自 LiliaUI；模板通过稳定的本地 facade 在构建期选择 Lilia 或 Nana，生产包只包含一个完整 Layer。业务代码从 `src/features`、`src/routes.ts` 和 `src/commands.ts` 开始扩展。

## 开始

```bash
yarn install
yarn dev
```

更多命令见[开发启动](./guide/development.md)。

- [Nana 产品能力](./guide/nana-product-layer.md)
- [验收矩阵](./guide/acceptance.md)
