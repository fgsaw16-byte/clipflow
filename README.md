# ClipFlow 🌊

> 这是一个采用 **Tauri v2 + React/TypeScript + Rust** 构建的轻量级 Windows 剪贴板管理器。极简的界面之下，隐藏着原生的系统级心脏。

## 🤖 AI 驱动开发探索
本项目的核心架构、底层 Win32 原生消息通信机制以及前端的交互（还在不断摸索阶段，现在还是较为生硬），均是在 AI 的深度辅助下结对编程完成的。作为一次 AI 赋能桌面端开发的深度实践，它既是对现代跨端技术的探索，也是一场赛博朋克式的开发实验。

## ✨ 核心特性
- **Win32 原生监听**：彻底抛弃高耗能的定时轮询，采用 Windows 底层 `AddClipboardFormatListener` 事件驱动，实现零 CPU 占用的静默监听。
- **双保险防死锁机制**：针对 Windows 休眠唤醒导致的“幽灵掉线” Bug，重构了底层 Mutex 锁机制与断链自动重连。
- **极简隐形 UI**：界面极度克制，引入了类似移动端的 `Pull-to-Refresh`（下拉刷新）阻尼物理动画，将后台强制同步指令优雅地隐藏在直觉交互中。
- **现代技术栈**：基于 Tauri v2 框架，享受 Rust 的内存安全与极致性能，同时拥有 React 赋予的绝美前端表现力。

## 🚀 快速启动
如果你想在本地运行或改进这个项目，请确保你已经安装了 Node.js 和 Rust 环境。

```bash
# 1. 克隆仓库
git clone [https://github.com/fgsaw16-byte/clipflow.git](https://github.com/fgsaw16-byte/clipflow.git)

# 2. 进入目录并安装前端依赖
cd clipflow
npm install

# 3. 启动开发服务器
npm run tauri dev
🤝 参与贡献
由于本项目带有强烈的个人探索性质，代码中难免存在一些不够优雅的实现或“薛定谔的 Bug”。
非常欢迎各位路过的大佬审阅源码、提交 Issue 探讨方案，或者直接 PR 指教。让我们一起把它打磨得更好！
