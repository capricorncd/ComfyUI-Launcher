# ComfyUI Launcher

## 许可证

本项目采用 [Apache License 2.0](LICENSE) 许可证。

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

## Timeline 项目保存

主窗口在 `http://127.0.0.1:8188` 的顶层页面注入 `window.__COMFYUI_LAUNCHER__`（`apiVersion: 1`），通过 `capabilities.projectDirectory` 检测原生目录选择能力。`pickDirectory()` 返回所选目录的绝对路径，取消时返回 `null`。该对象用于能力检测，不作为文件访问的授权凭据；远程页面仅获准调用目录选择命令。

配合新版 Capricorncd Timeline，从「导入 → 从目录导入」打开项目后，自动保存和关闭编辑器时写入原目录的 `project.json.bak` / `storyboard.json.bak`；Ctrl+S 或时间轴「更多 → 保存项目」覆盖正式的 `project.json` / `storyboard.json`。「更多 → 打开项目目录」打开该目录。首次导出为目录的新项目会关联导出的目录；已有项目的导出不改变保存位置，ZIP 导出不建立目录关联。

合成视频、Clip 视频及音频导出可选择其他目录，默认仍为 output。所有导出共用上次成功导出的目录，并保存在本机浏览器偏好中；目录和桌面会话不写入 workflow、project 或 storyboard JSON。

保存时素材以原始字节复制到 `media/launcher/`，按内容命名并复用，避免备份修改正式项目引用的素材。保存失败会在编辑器显示错误并允许重试。关联保留在当前节点的运行会话中，刷新页面或重启 ComfyUI 后需重新导入目录。恢复备份时应同时恢复两个 `.bak` 文件。此功能需重新编译并启动 launcher，以及重启 ComfyUI 加载新版后端。

## 运行日志

File → 打开 → 日志目录可查看日志（Windows：`%LOCALAPPDATA%\com.comfyuilauncher.app\logs`）。按 UTC 日期保存 `comfyui-YYYY-MM-DD.log`，保留当天及前 6 天，启动和跨天写入时清理旧日志。记录标准输出、标准错误、启动路径、进程号和退出状态；Python 使用无缓冲输出并启用故障堆栈。进程退出后返回错误页。已在外部启动的 ComfyUI 无法采集输出，需通过启动器启动。
