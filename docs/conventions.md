# VisualASM 项目约定

本文档定义 VisualASM 仓库的工程约定，所有代码与文档变更应遵守。

## 1. 项目定位

VisualASM 是一个面向**王爽《汇编语言》教材**（清华出版社，目标 Intel 8086 实模式）
的 TUI 工具。核心能力：解析教材风格的 `.asm` 源文件 → 解释执行 → 在 TUI 中实时展示
段寄存器、通用寄存器、内存、栈、标志位、当前指令的变化。

设计灵感来自 DOS 下的 `debug.exe`，目标是把 `debug` 那种"看得见每条指令效果"
的体验现代化、跨平台化。

**当前阶段非目标**：
- 不做 32/64 位（保留为未来版本，见 §3 的代码组织约定）。
- 不实现完整 DOS/BIOS；`int 21h` / `int 10h` / `int 16h` 仅按教材使用到的功能号 stub。
- 不做 MASM 链接器/重定位；输入是单文件源码，伪指令仅覆盖教材所需子集。
- 不做反汇编（输入是文本汇编源文件，不是 .COM/.EXE 二进制）。

## 2. 目标 ISA 与语法

- **CPU**：Intel 8086（16 位寄存器 ax/bx/cx/dx/si/di/bp/sp、段寄存器 cs/ds/ss/es、ip、flags）。
- **内存模型**：分段，物理地址 = 段 × 16 + 偏移；地址空间上限 1 MiB（教材常用 64 KiB 段足够）。
- **语法**：MASM 风格 Intel 语法，对齐教材写法（`assume`、`segment ... ends`、`end start`）。
- 立即数：十进制、`H` 后缀十六进制（`0a000h`，首位是字母时前导 `0`）、`B` 后缀二进制、字符常量 `'A'`。
- 注释：`;` 到行尾。
- 行结束符：lexer 与文件读取必须同时接受 `\n` 与 `\r\n`；行号按"逻辑行"计算。
- 不支持宏、`include`、结构体定义、`proc/endp`、`if/endif` 条件汇编。

### VM 字节流编码策略

- `vm::dos` / `vm::bios` 的字符输入输出在 VM 内部一律**按字节**存（与 8086 时代一致，常见为 ASCII / GBK / CP437），不允许在 VM 层假设任何编码。
- 编码转换只发生在 `ui::panes::console` 渲染时与 Console 输入入栈时；策略由 CLI flag 控制（默认 GBK，详见 `plan.md` M5）。
- 这条约束让 VM 行为完全确定、可被字节级测试；同时让 Console 面板在不同平台显示一致。

## 3. 技术栈与依赖

| 领域      | 选型                                          | 说明                              |
| --------- | --------------------------------------------- | --------------------------------- |
| 语言      | Rust 2024 edition                             | Cargo.toml 已锁定                 |
| TUI       | `ratatui` + `crossterm`                       | 不引入其他渲染后端                |
| 解析      | 手写 lexer + 递归下降 parser                  | 不引入 nom/pest，便于教学         |
| 错误处理  | `thiserror`（库）/ `anyhow`（二进制入口）     | 不裸用 `Box<dyn Error>`           |
| 日志      | `tracing` + `tracing-subscriber`，写文件      | 不污染 TUI                        |
| CLI       | `clap` derive 风格                            |                                   |
| 测试      | `cargo test` + `insta` 快照                   |                                   |

依赖增加须在 PR 描述中说明用途；优先选纯 Rust、零 unsafe 的成熟 crate。

## 4. 目录结构

```
src/
  main.rs              # 入口，仅做参数解析 + 装配
  cli.rs
  error.rs
  app/                 # TUI 状态机、事件循环、键位绑定
    mod.rs
    event.rs
    keymap.rs
  ui/                  # 纯渲染层，无业务状态写入
    mod.rs
    panes/             # registers / segments / flags / memory / stack / source / disasm
  asm/                 # 汇编前端（与 ISA 解耦的语法层）
    lexer.rs
    parser.rs
    ast.rs
    diagnostics.rs
  vm/                  # 执行引擎
    mod.rs
    i8086/             # 8086 实现：cpu / memory / exec / isa/*
      cpu.rs
      memory.rs        # 段式寻址、物理地址换算
      exec.rs
      isa/             # 每族指令一个文件
    bios/              # int 10h / 16h stub
    dos/               # int 21h stub
  trace/               # 单步、历史、可回放
tests/
  fixtures/*.asm
  snapshots/
docs/
  plan.md
  conventions.md
examples/              # 教材章节配套示例
```

约束：
- `ui/` 只读 `app` 的状态，不允许反向引用 `vm`。
- `vm/` 不依赖 `ratatui`、`crossterm`，必须可被无 TUI 单元测试驱动。
- `asm/` 不依赖 `vm/`；AST 是与执行无关的纯数据结构。
- **为未来 64 位版本预留位置**：CPU 实现统一放在 `vm/<arch>/` 下（当前只有 `i8086/`）；
  上层 `vm/mod.rs` 暂时直接 `pub use i8086::*`，**不要现在就抽 trait**——等真的开始做
  64 位时再按差异引入最小抽象。过早抽象成本远高于到时重构。

### UI 约定

- TUI 布局为五行：状态栏 / 三栏主区（源码 · 控制台 · 寄存器组）/ 内存独占一行 / 底部解释 + 键位。详见 `plan.md` M3.1。
- 源码区**只读**；编辑通过 `e` 调起 `$EDITOR`，退出后由 `app` 重新加载并重置 VM。
- 焦点模型分两种模式：
  - **控制模式**（除 Console 外的任何面板获得焦点）—— 单字符键触发 VM 动作（`s/n/c/b/r/u/g/e/q` 等）。
  - **输入模式**（Console 焦点）—— 按键作为 8086 程序的标准输入（喂给 `int 21h` / `int 16h`），`Esc` 退出。
  - 当前模式与焦点面板名必须显示在顶部状态栏，防止误操作。
- `vm/` 的指令实现应附带"语义元数据"（伪代码模板 + 影响的 flags），供 `ui::panes::explain` 渲染。元数据是声明式的，不允许在 UI 层硬编码每条指令的解释文案。

## 5. 代码风格

- 遵循 `rustfmt` 默认配置，`cargo fmt --check` 必须通过。
- `cargo clippy --all-targets -- -D warnings` 必须通过。
- 模块/文件名小写下划线；类型 `UpperCamel`；函数与变量 `snake_case`；常量 `SCREAMING_SNAKE`。
- `unsafe` 默认禁止；如必须引入，在该函数上方写明安全前置条件。
- 公共 API 写 doc comment；私有项默认不写注释，除非"为什么"非显然。
- 错误类型集中放在所属模块的 `error.rs` 或同文件末尾，禁止把字符串当错误抛。
- 8086 是 16 位机器，VM 内部所有寄存器/地址都用 `u16` / `u32`（物理地址），
  **禁止用 `usize` 表达机器层概念**，防止 32/64 位移植时语义漂移。

## 6. 测试

- `vm/i8086/isa/` 下每个指令文件配套单元测试，覆盖：正常路径、边界（溢出/进位/零/符号）、错误路径。
- `asm/parser.rs` 用 `insta` 对典型 AST 做快照测试，输入 fixture 同时覆盖 LF 与 CRLF 两种行结束符。
- TUI 不做像素级测试；面板渲染函数应能脱离终端被调用并断言其 `ratatui::buffer::Buffer` 输出。
- 端到端：`tests/fixtures/` 下放教材风格的 `.asm`，跑完后断言最终 CPU 与内存状态。
- 教材习题应尽量收成 fixture，构成长期回归基准。
- 涉及纯 Unix 行为（如 `$EDITOR` 默认值、文件权限）的测试允许标 `#[cfg(unix)]`；但**禁止**因为图省事就让 Windows 上整条测试被跳过——核心逻辑测试必须三平台都跑。

## 7. Git 与提交

- 默认分支 `main`，开发分支 `feat/<scope>`、`fix/<scope>`、`docs/<scope>`。
- Commit 信息采用 Conventional Commits：`feat(vm): add loop instruction`。
- **Commit message 一律不写 `Co-Authored-By` 或其他工具/AI 署名**——保持 history 干净。**用中文**
- 单个 PR 聚焦一个目的；不要把无关重构混入功能 PR。
- 不直接 push 到 `main`；通过 PR 合并。

## 8. 文档

- 任何新增指令、键位、面板都需要更新本目录下相应文档。
- `docs/plan.md` 是路线图，里程碑完成后勾掉对应项并写完成日期。
- 用户可见的行为变更需在 README（后续创建）中提及。
- 涉及教材章节的功能，PR 描述中标明对应章节号（如"对应教材 §9"），便于读者对照。

## 9. 平台支持

- v1 一等公民平台：**Linux、macOS、Windows** 三者完全等同。任一平台 broken 都视为 release blocker。
- CI 必须在 `ubuntu-latest` + `macos-latest` + `windows-latest` 三平台全绿才能合并。
- 不允许引入 platform-specific 代码路径而无回退；若不得不写 `#[cfg(windows)]` / `#[cfg(unix)]`，必须两侧都实现等价语义。
- 终端能力假设：crossterm 0.28 在 Windows 下走 ConPTY，最低 Windows 10 1809；README 中需注明。
- 字符编码：见 §2 「VM 字节流编码策略」——VM 不做编码假设，UI 渲染层一次性转码。
