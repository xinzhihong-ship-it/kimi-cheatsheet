# Kimi Cheatsheet

Kimi Code CLI 命令速查工具 —— macOS 菜单栏小应用。

> 本项目全部代码由 Kimi 生成。

![platform](https://img.shields.io/badge/platform-macOS-blue)
![release](https://img.shields.io/github/v/release/xinzhihong-ship-it/kimi-cheatsheet)

## 功能

- 📋 **命令速查**：整理常用 Kimi Code CLI 命令，点击即可复制到剪贴板
- 🔍 **快速搜索**：支持按命令或中文说明搜索
- 📊 **额度查询**：菜单栏实时显示 Kimi Token 额度剩余情况
- 🚀 **一键启动**：右键菜单快速启动 Kimi CLI / Web UI
- ⬆️ **版本检测**：自动检测 Kimi Code CLI 最新版，有新版本时菜单栏图标变红
- 🔧 **自定义命令**：可增删改命令列表，支持导出/导入

## 安装

1. 从 [Releases](https://github.com/xinzhihong-ship-it/kimi-cheatsheet/releases) 下载最新 `.dmg`
2. 打开 `.dmg`，将 `Kimi Cheatsheet.app` 拖到 **应用程序** 文件夹
3. 首次启动后，在菜单栏找到 **Kimi** 图标即可使用

## 使用说明

| 操作 | 说明 |
|------|------|
| 左键点击菜单栏图标 | 显示/隐藏主窗口，同时刷新额度 |
| 右键点击菜单栏图标 | 打开功能菜单 |
| 点击命令 | 复制到剪贴板 |
| 底部「项目」按钮 | 打开 Kimi Code 官方 GitHub |
| 底部「升级」按钮 | 打开 Terminal 执行 `kimi upgrade` |

## 开发构建

```bash
cd kimi-cheatsheet-rust/src-tauri
cargo-tauri build
```

构建产物：`src-tauri/target/release/bundle/macos/Kimi Cheatsheet.app`

## 数据来源

- Kimi Code CLI: https://github.com/MoonshotAI/kimi-code
- 额度接口：Kimi 官方 API

## License

MIT
