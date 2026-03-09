# symi-rs

`symi-rs` 是 `symi` 语言的 Rust 核心库，负责：

- 词法分析与语法树构建（基于 `logos` + `rowan`）
- 语法树编译为时间化音乐事件（`CompileEvent`）
- 将编译事件导出为标准 MIDI（SMF Format 1）
- 提供基于 `glicol_synth` + `cpal` 的实时音频播放能力

> 当前包名：`symi-rs`，库名：`symi`

---

## 环境要求

本库在 `src/lib.rs` 中启用了以下特性：

- `iter_from_coroutine`
- `coroutines`
- `yield_expr`

因此需要 **Rust nightly**。

建议：

```bash
rustup toolchain install nightly
rustup override set nightly
```

---

## 安装

```toml
[dependencies]
symi = { package = "symi-rs", version = "0.1.5" }
```

---

## 快速开始

### 1) 解析 + 编译 Symi 源码

```rust
use std::sync::Arc;
use symi::{parse_source, Compiler};

fn main() {
		let source: Arc<str> = Arc::from("(4/4)\n(120)\nC4:E4,\n");

		let parsed = parse_source(source);
		let mut compiler = Compiler::new();
		compiler.compile(&parsed.syntax_node());

		println!("events: {}", compiler.events.len());
		println!("diagnostics: {}", compiler.diagnostics.len());
}
```

### 2) 导出 MIDI（SMF Format 1）

```rust
use std::{fs, sync::Arc};
use symi::{parse_source, Compiler};
use symi::midi::writer::{export_smf_format1, MidiWriterConfig};

fn main() -> anyhow::Result<()> {
		let source: Arc<str> = Arc::from("(4/4)\n(120)\nC4:E4,\n");
		let parsed = parse_source(source);

		let mut compiler = Compiler::new();
		compiler.compile(&parsed.syntax_node());

		let bytes = export_smf_format1(&compiler.events, MidiWriterConfig::default())?;
		fs::write("out.mid", bytes)?;
		Ok(())
}
```

### 3) 实时播放单音（异步）

```rust
use symi::glicol::audio::AudioHandle;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
		let audio = AudioHandle::new()?;
		audio.set_volume(0.5);
		audio.play_note(440.0, 0.5).await;
		Ok(())
}
```

---

## 对外 API（核心）

`src/lib.rs` 重新导出了以下常用入口：

- `parse_source(source: Arc<str>) -> Parse`
- `Compiler`（`compile(&SyntaxNode)`）
- `SyntaxKind`
- `Parse`
- `compiler::types::*`
- `glicol::audio::*`

常见结构：

- `Compiler`
	- `events: Vec<CompileEvent>`：编译后的事件流
	- `diagnostics: Vec<Diagnostic>`：编译诊断（错误/警告）
	- `macros: MacroRegistry`：宏注册表（alias/simple/complex）
- `EventBody`
	- `Note(Note)`
	- `BaseNoteDef` / `BaseFequencyDef`
	- `TimeSignatureDef` / `BeatDurationDef` / `BPMDef`
	- `QuantizeDef` / `NewMeasure`

---

## 语法能力概览

从 `lexer`、`parse_fn` 与编译测试可见，当前支持：

- 音高类型：
	- 音名（如 `C4`、`Db3`）
	- 频率（如 `440.0`）
	- 比率（如 `3/2`）
	- EDO（如 `7\12`）
	- 音分（如 `100c`）
	- 休止（`.`）与延音（`-`）
- 音高链：`@` 连接、`+/-` 八度后缀
- 宏系统：别名宏、简单宏、多行复杂宏
- 节拍/速度：拍号、拍时值、BPM
- 量化与时值标记：`{...}`、`[...]`

> 更完整语法说明建议配合仓库 `docs/` 子项目阅读。

---

## 目录结构

```text
src/
	lib.rs                 # 对外导出
	compiler/              # 编译器（语法树 -> 事件流）
	rowan/                 # lexer/parser/sink/types
	midi/                  # MIDI 导出
	glicol/                # 实时音频播放
	tests/                 # .symi 样例与快照文本
```

---

## 开发与测试

在 `rs/` 目录下执行：

```bash
cargo test
```

主要测试分布：

- `src/rowan/parse_fn.rs`：语法解析测试
- `src/compiler/compile.rs`：编译语义与诊断测试
- `src/midi/writer.rs`：MIDI 导出测试
- `src/tests/*.symi`：样例输入与快照输出

---

## License

Apache-2.0
