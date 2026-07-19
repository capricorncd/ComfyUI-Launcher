# ComfyUI Launcher

基于 Tauri v2（Rust + WebView）的 ComfyUI 桌面启动器。程序启动后自动拉起本地 ComfyUI（`python_embeded\python.exe -s ComfyUI\main.py`），等待其就绪后在主窗口中直接显示 ComfyUI 网页界面；窗口原生菜单栏提供 **File** 菜单，包含：

- **启动/重启** — 结束当前 ComfyUI 进程（含子进程）并重新启动
- **自定义节点管理** — 列出 `custom_nodes` 下所有节点的版本 / 最近更新时间，支持一键 `git pull` 更新，以及粘贴 GitHub 地址后 `git clone` 新节点（已存在的仓库会自动提示改用更新）
- **打开自定义节点目录 / 打开输出目录 / 打开模型目录** — 在资源管理器中打开对应目录
- **设置** — 配置 ComfyUI 安装根目录；首次启动或路径无效时会自动弹出该窗口

## 环境要求

- [Node.js](https://nodejs.org/)（建议 v20+）与 npm
- [Rust](https://www.rust-lang.org/tools/install)（stable 工具链，`cargo`/`rustc`）
- Windows：需要 [WebView2 运行时](https://developer.microsoft.com/microsoft-edge/webview2/)（Windows 10/11 通常已内置）
- 本地已安装好的 ComfyUI 便携版目录（包含 `python_embeded\python.exe` 与 `ComfyUI\main.py`），以及可用的 `git`（自定义节点管理功能依赖 `git` 命令）

## 开发调试

```bash
npm install
npm run tauri dev
```

`tauri dev` 会先启动 Vite 开发服务器，再编译并运行 Rust 后端，源码改动（`src/`、`src-tauri/src/`）会自动触发热更新 / 重新编译重启。

首次运行、或配置的路径下找不到 `python_embeded\python.exe` / `ComfyUI\main.py` 时，会自动弹出「设置」窗口，填写 ComfyUI 安装根目录并保存即可自动启动。

配置文件保存在系统配置目录下的 `com.comfyuilauncher.app/config.json`（Windows 一般在 `%APPDATA%\com.comfyuilauncher.app\config.json`）。

仅编译检查 Rust 后端（不启动界面）：

```bash
cd src-tauri
cargo build
```

## 编译打包

```bash
npm run tauri build
```

该命令会执行前端构建（`tsc && vite build`）并编译 Rust 后端的 release 版本，产物位于：

- 可执行文件：`src-tauri/target/release/`
- 安装包（Windows 下为 `.msi` / `.exe` 安装程序）：`src-tauri/target/release/bundle/`

打包前可在 `src-tauri/tauri.conf.json` 中调整 `productName`、`version`、`bundle.icon` 等信息。

## 项目结构

```
comfyui-launcher/
  index.html / src/main.ts        # 主窗口加载页（启动状态 / 日志 / 失败重试）
  nodes.html / src/nodes.ts       # 自定义节点管理窗口
  settings.html / src/settings.ts # 设置窗口
  src/styles.css                  # 共用样式
  src-tauri/
    src/
      lib.rs        # 应用入口：初始化状态、菜单、自动启动 ComfyUI
      config.rs      # 配置读写与校验（ComfyUI 根目录）
      process.rs      # ComfyUI 进程启停、日志采集、就绪轮询
      nodes.rs        # 自定义节点扫描 / git pull / git clone
      windows.rs       # 子窗口开关、目录打开
      menu.rs         # File 原生菜单构建与事件分发
      commands.rs      # 暴露给前端的 Tauri 命令
      state.rs        # 全局共享状态（配置、进程状态、日志缓冲）
    tauri.conf.json  # 窗口 / 打包配置
    capabilities/     # 前端可调用的插件权限声明
```
