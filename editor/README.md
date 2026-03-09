# Symi Editor

`Symi Editor` 是一个桌面文本编辑器，用于编写 Symi 乐谱语言，并提供：

- 语法高亮与诊断
- 事件可视化（钢琴卷帘）
- 实时试音（播放单音）
- 导出标准 MIDI（SMF Format 1）

前端基于 `Nuxt 4 + Vue 3 + CodeMirror`，桌面端基于 `Tauri 2`，语义解析与编译来自同仓库的 `symi-rs`。

---

## 功能概览

- 多标签文件管理（新建 / 打开 / 保存 / 另存为 / 关闭）
- 文件状态持久化（IndexedDB）
- CodeMirror 编辑器与语法扩展
- 诊断信息（解析错误 + 编译错误/警告）
- 钢琴卷帘视图（跟随编辑状态）
- 音量控制与播放
- MIDI 导出参数配置：
	- Pitch Bend Range (RPN)
	- TPQ（ticks per quarter）
	- 时间容差（秒）
	- 音高容差（音分）

---

## 环境要求

### 必需

- `Bun`（项目 `packageManager` 为 `bun@1.3.9`）
- `Rust`（`src-tauri/Cargo.toml` 指定 `rust-version = 1.77.2`）
- Tauri 2 依赖环境（各平台原生依赖）

### Windows 说明

- Tauri 打包/运行需要 WebView2（通常系统已安装）

---

## 安装依赖

```bash
bun install
```

---

## 开发

### 仅前端开发（Nuxt）

```bash
bun dev
```

- 默认地址：`http://localhost:3461`

### 桌面开发（Tauri + Nuxt）

```bash
bunx tauri dev
```

说明：Tauri 配置中已设置 `beforeDevCommand: "bun dev"`，会自动拉起前端开发服务。

---

## 构建

### 前端静态构建

```bash
bun run generate
```

### 桌面应用打包

```bash
bunx tauri build
```

说明：Tauri 配置中已设置 `beforeBuildCommand: "bun run generate"`。

---

## 常用快捷键

- `Ctrl+S`：保存当前文件
- `Ctrl+Shift+S`：另存为
- `Ctrl+O`：打开文件
- `Ctrl+N`：新建文件
- `Ctrl+W`：关闭当前标签
- `Ctrl+Space`：播放 / 暂停（编辑区）
- `Ctrl+/`（macOS: `Cmd+/`）：注释切换

钢琴卷帘视图支持滚轮滚动/缩放、空格播放等交互（见“帮助”弹窗）。

---

## MIDI 导出流程

1. 在侧栏点击“导出MIDI”
2. 选择输出路径（`.mid/.midi`）
3. 调整导出参数（RPN、TPQ、容差）
4. 点击导出

导出前会先调用后端校验（解析/编译/导出参数），校验通过后才写入文件。

---

## 项目结构

```text
app/
	components/            # 编辑器、侧栏、状态栏、钢琴卷帘、导出弹窗等
	composables/           # 卷帘逻辑、分栏拖拽、滚轮平滑滚动
	utils/                 # 文件标签管理、CodeMirror 扩展

src-tauri/
	src/commands.rs        # 前端 invoke 命令（诊断、事件、播放、导出）
	src/manager.rs         # 文件与音频管理器
	src/lib.rs             # Tauri 入口与命令注册
```

---

## 与 `symi-rs` 的关系

桌面后端通过 `src-tauri/Cargo.toml` 中的路径依赖：

```toml
symi-rs = { path = "../../rs" }
```

也就是说，编辑器的解析、编译、MIDI 导出能力直接来自本仓库 `rs/` 子项目。

---

## License

Apache-2.0
