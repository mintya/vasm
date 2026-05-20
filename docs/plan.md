# VisualASM 实现计划

本文档是 VisualASM 的分阶段实现路线图。每个里程碑给出目标、范围、产物与验收标准。
约定见 [conventions.md](./conventions.md)。

## 总览

VisualASM v1 对齐**王爽《汇编语言》8086 实模式**。指令子集与里程碑按教材章节推进，
完成 v1 后再启动 v2（x86-64）。

```
M0 脚手架  →  M1 汇编前端  →  M2 VM 内核（教材 §2~7）→  M3 TUI 骨架
                                                            ↓
M7 v1 发布  ←  M6 教材高阶（§16~17）←  M5 中断与 I/O（§12~15）←  M4 控制流与标志（§8~11）
                                                            ↓
                                                        v2 路线（x86-64，独立文档）
```

目标产出：一个 `vasm <file.asm>` 命令，打开 TUI，左侧源码、右侧段/寄存器/标志/内存/栈，
支持单步、断点、回退、`int 21h` / `int 10h` / `int 16h` 的核心功能号；
能跑通王爽教材正文与课后实验中绝大多数示例。

---

## M0 · 项目脚手架

**目标**：能 `cargo run` 起一个空 TUI 并通过 `q` 退出。

- [x] 引入依赖：`ratatui`、`crossterm`、`clap`、`anyhow`、`thiserror`、`tracing`、`tracing-subscriber`、`insta`（dev）。
- [x] 建立 `conventions.md` §4 定义的目录结构（占位 `mod.rs` 即可）。
- [x] `src/cli.rs`：`vasm <file>`、`--log <path>`、`--mem-kb <N>`（默认 1024）。
- [x] `app::run()` 进入 alternate screen + raw mode；`q` / `Ctrl-C` 退出并恢复终端。
- [x] CI（GitHub Actions）：`ubuntu-latest` + `macos-latest` + `windows-latest` 三平台 matrix，每个跑 `fmt`、`clippy -D warnings`、`test`。

**验收**：执行 `vasm anyfile.asm` 出现一个标题为 `VisualASM` 的空 TUI 边框，按 `q` 干净退出且不破坏终端；CI 三平台 job 全绿。

---

## M1 · 汇编前端（Lexer + Parser + AST）

**目标**：把 MASM 风格 `.asm` 文本变成结构化 AST，并产出可读的诊断。

- [x] `asm::lexer`：标识符、数字字面量（10/16/2 进制，带 `H/B` 后缀）、字符与字符串常量、寄存器名、标点、注释（`;`）。
- [x] `asm::ast`：
  - `Program { segments, assume, entry }`
  - `Segment { name, items: Vec<Item> }`，`Item = Label | Directive | Instruction`
  - `Operand = Reg | Seg | Imm | Mem | Label | SegOverride`
  - `Mem` 支持 `[bx]`、`[bx+si]`、`[bx+idata]`、`[bx+si+idata]`、`ds:[...]` 等教材寻址形式。
- [x] `asm::parser`：手写递归下降，覆盖 M2~M6 指令所需语法。
- [x] **伪指令**：`segment/ends`、`assume`、`db/dw/dd`、`dup`、`offset`、`end <label>`、`org`（教材偶尔用）。
- [x] `asm::diagnostics`：`Span { line, col, len }`，错误信息能定位到列。
- [x] 用 `insta` 对 5~10 个教材风格 fixture 做 AST 快照测试。

**验收**：能解析教材 §3 / §4 / §6 的代表程序并输出与快照一致的 AST；语法错误时 stderr 打印 `file:line:col: error: ...`。

---

## M2 · VM 内核（对应教材 §2~§7）

**目标**：脱离 TUI 即可解释执行 AST，得到确定的最终状态。

### M2.1 状态模型

- [ ] `vm::i8086::cpu::Cpu`：
  - 通用寄存器 ax/bx/cx/dx（含 ah/al 等 8 位别名）、si/di、bp/sp
  - 段寄存器 cs/ds/ss/es
  - ip、flags（CF/PF/AF/ZF/SF/TF/IF/DF/OF）
- [ ] `vm::i8086::memory::Memory`：1 MiB 线性数组；`phys(seg, off) = seg << 4 + off`；`read/write u8/u16` + 越界错误（教材会绕回，但默认报错可在 `--lenient` 下降级为绕回，方便排错）。
- [ ] `vm::i8086::exec::Vm`：组合 cpu+memory+program；`step()` 返回 `StepOutcome { effects, next_ip, halted }`。
- [ ] **段加载器**：按 `segment` 顺序铺到内存；`end start` 决定入口段:偏移；`assume` 仅记录辅助信息，不参与执行（教材语义）。

### M2.2 指令首批（教材 §2~§7 必用）

- 数据传送：`mov`（reg/mem/imm/seg 互相组合）、`xchg`、`push`、`pop`、`pushf`、`popf`
- 算术：`add`、`sub`、`inc`、`dec`
- 循环：`loop`（仅根据 `cx` 递减）
- 寻址：`[bx]`、`[bx+si]`、`[bx+di]`、`[bp+...]`（默认段为 ss）、立即偏移、段超越 `ds:`/`es:`/`ss:`/`cs:`
- 段相关：`mov ds, ax`（保留教材"段寄存器不能直接 mov 立即数"的限制并给出诊断）

实施约束：
- `vm/i8086/isa/` 每族一个文件，每个指令一个 `fn`；用 `match Mnemonic` 分派。
- 标志位按 Intel 8086 行为；写测试时引用 Intel 80186 手册较 SDM 更对位。
- 每条指令配套单元测试（正常 + 边界 + 错误）。

**验收**：能跑通 `examples/ch06_multi_segment.asm`（教材 §6 多段程序），最终内存与寄存器状态正确。

---

## M3 · TUI 骨架

**目标**：把 VM 状态搬到屏幕上，但暂不支持交互式控制（自动一次跑完，最后展示终态）。

### M3.1 整体布局

五行结构，自上而下：

```
┌─ vasm  examples/ch10_call.asm   ● Paused   cs:ip=076A:0012   #steps=42  x1 ─┐  ← 1. 状态栏（1 行）
├──────────────────────┬──────────────────────┬────────────────────────────────┤
│ Source         [F1]  │ Console        [F2]  │ Registers              [F3]   │  ← 2. 三栏主区
│  10  start:          │ Hello, world!_       │ ax=...  bx=...                │     （自适应高度）
│  11    mov ax,1      │                      │ ...                           │
│▶ 12    add ax,bx     │                      ├─ Segments ────────────────────┤
│● 13    call sum      │                      │ cs=...  ds=...                │
│  ...                 │                      ├─ Flags ───────────────────────┤
│                      │                      │ CF·  ZF✓  SF·  OF·  ...       │
│                      │                      ├─ Stack (ss:sp) ───────────────┤
│                      │                      │ ...                           │
├──────────────────────┴──────────────────────┴─ Call Stack ───────────────────┤  ← 3. 调用栈嵌在右栏底部
│ Memory  ds:0000                                                              │  ← 4. 内存独占一行
│ 0000  48 65 6C 6C 6F 2C 20 77  6F 72 6C 64  21 00 00 00   Hello, world!.... │     （高度 4~8 行可调）
│ 0010  ...                                                                    │
├──────────────────────────────────────────────────────────────────────────────┤
│ ▶ add ax, bx   ;  ax ← ax + bx  =  1 + 4  =  5    flags: ZF=0 SF=0 CF=0     │  ← 5. 底部双行
│ [s]tep [n]ext [c]ontinue [b]reak [r]eset [u]ndo [g]oto [e]dit [Tab] [q]uit  │
└──────────────────────────────────────────────────────────────────────────────┘
```

### M3.2 实现项

- [ ] `app::App`：持有 `Vm` 与渲染所需投影状态；`tick/event` 双循环。
- [ ] `ui::panes::status`：顶部状态栏。字段：文件名、运行状态（Running/Paused/Halted/Error）、`cs:ip`、已执行指令计数、运行倍速、**当前焦点面板名**（防止用户在 Console 焦点下误按控制键）。
- [ ] `ui::panes::source`：**只读**查看器；语法高亮（关键字、寄存器、立即数、注释、字符串、标签）；当前 `cs:ip` 行 `▶` 标记；断点行 `●` 标记。按 `e` 调起外部编辑器（回退链：Unix `$EDITOR → $VISUAL → vi`；Windows `%EDITOR% → %VISUAL% → notepad.exe`），退出后自动重载并重置 VM。GUI 编辑器需要用户自行在环境变量里加 `--wait` 之类参数，README 给提示。
- [ ] `ui::panes::console`：DOS 字符输出缓冲（M5 才会有真实写入）；获得焦点时所有按键作为程序输入，**焦点状态在状态栏显式可见**。
- [ ] `ui::panes::registers`：通用寄存器 + ip；变化的寄存器高亮一次。
- [ ] `ui::panes::segments`：cs/ds/ss/es 及其当前指向段名。
- [ ] `ui::panes::flags`：CF/PF/AF/ZF/SF/TF/IF/DF/OF 状态点。
- [ ] `ui::panes::stack`：以 `ss:sp` 为基准上下展示 N 个 word，`sp` 指向位置高亮。
- [ ] `ui::panes::call_stack`：逻辑调用栈（M3 阶段先占位渲染，真正维护在 M4 `call/ret` 实现时补齐）。
- [ ] `ui::panes::memory`：横向独占一行，可指定 `seg:off` 起点的 hex+ASCII dump（默认 `ds:0`）；支持上下滚动；`g` 跳转到地址。
- [ ] `ui::panes::explain`：底部第 1 行，显示即将执行指令的等价伪代码 + 数值代入 + 即将变化的 flags（M3 用占位文本，元数据驱动在 M5/M6 完善）。
- [ ] `ui::panes::keymap`：底部第 2 行，根据当前焦点显示对应键位（Console 焦点下只显示"Esc 退出输入"）。

### M3.3 焦点模型

- 全局焦点环：`Source → Console → Registers/Segments/Flags/Stack/CallStack → Memory`，`Tab` 顺序切换，`Shift+Tab` 反向。
- **Console 焦点是"输入模式"**，键盘事件交给 VM 输入缓冲；其他面板焦点都是"控制模式"，按 `s/n/c/b/...` 触发动作。
- 进入 Console 焦点用 `F2` 或 `Tab` 切到；退出用 `Esc`。
- 任何模式下 `Ctrl-C` 都能干净退出（防呆）。

**验收**：`vasm examples/ch06_multi_segment.asm` 启动后能看到完整布局与终态数据，焦点切换正确，按 `e` 能调起外部编辑器并在退出后重载，`q`（控制模式下）退出。

---

## M4 · 控制流与标志位（对应教材 §8~§11）

**目标**：让程序能"做判断"，并支持交互式调试。

### M4.1 指令扩展

- 算术/逻辑：`mul`、`div`、`and`、`or`、`xor`、`not`、`neg`、`shl`、`shr`、`sal`、`sar`、`rol`、`ror`、`rcl`、`rcr`
- 比较：`cmp`、`test`
- 跳转：`jmp short/near/far`、`jmp word ptr ds:[...]`（教材 §16 会用）、`jcxz`
- 条件跳转：`je/jne/jz/jnz/jg/jge/jl/jle/ja/jae/jb/jbe/jo/jno/js/jns/jp/jnp`
- 过程：`call`（near/far）、`ret`、`retf`

### M4.2 交互（写入 `app::keymap`，conventions 同步）

- `s` 单步执行（step into）
- `n` 步过（call 视为一步）
- `c` 连续运行至断点或终止
- `b` 在源码光标处设/取消断点
- `r` 重置 VM 到初始状态
- `g` 跳到 `seg:off` 或标签（弹出输入框）
- `Tab` 切换焦点面板；`↑↓PgUp/PgDn` 在焦点面板内滚动
- 断点容器；执行循环中检查命中。
- 源码区显示断点标记 `●` 与当前指令 `▶`。

**验收**：能在教材 §9~§11 示例（带 cmp/条件跳转/call ret）上单步、设断点、命中后停下；面板焦点切换不丢状态。

---

## M5 · 中断与 I/O（对应教材 §12~§15）

**目标**：让程序能"看见"输入输出，跑通教材里所有 `int 21h` / `int 10h` / `int 16h` 示例。

- [ ] `int` 指令：识别中断号并查表分派到 stub；保留中断向量表的逻辑结构（位于 `0:0`），允许程序读写但 stub 路径不实际解引用。
- [ ] `iret`：仅在用户自定义中断处理程序里有意义；M5 阶段实现完整压栈/弹栈语义。
- [ ] `vm::dos`：`int 21h` 至少实现教材使用到的功能号
  - `01h` 键盘输入回显、`02h` 显示单字符、`09h` 显示 `$` 结尾字符串、`0Ah` 缓冲键盘输入
  - `4Ch` 退出（驱动 VM 停机）
- [ ] **DOS 字符编码**：VM 内部一律按字节存（不假设编码）；Console 面板渲染与输入时做编码转换。CLI 增加 `--encoding utf8|gbk|cp437`，默认 `gbk`（与王爽教材中文字符串期望一致）。
- [ ] `vm::bios`：
  - `int 10h`：`00h` 设置显示模式（仅记录）、`02h` 设置光标位置、`09h/0Ah` 字符输出、`13h` 字符串输出
  - `int 16h`：`00h/01h` 键盘读取（从 TUI 输入缓冲取）
- [ ] `in`、`out`：教材 §14 用到，但仅对少数已知端口 stub（如 60h/61h/40h 定时器），其他端口报"未实现"。
- [ ] 外中断（§15）：以"事件注入"形式支持，TUI 可手动触发一次特定 IRQ；不做真实异步。
- [ ] `cli`、`sti`：维护 IF 标志，不影响 stub 路径。

**验收**：能跑通教材 §12 用户中断、§13 字符串显示与键盘输入示例；TUI output 面板正确显示 DOS 输出。

---

## M6 · 教材高阶与可用性（对应教材 §16~§17 + 打磨）

- [ ] 直接定址表（§16）：`jmp word ptr ds:[bx+...]`、`call word ptr ...` 的间接跳转/调用，依托 M4 已实现的语法但要补 fixture。
- [ ] BIOS 键盘与磁盘（§17）：`int 16h` 高级功能、`int 13h` 仅 stub 到读扇区的最小语义（提供一个虚拟 1.44MB 磁盘镜像参数 `--disk a.img`）。
- [ ] 执行历史：每步保存 diff（被改寄存器/内存的旧值）；`u` 回退一步、`U` 回退到上一断点。
- [ ] 观察点 (watchpoint)：对寄存器或 `seg:off` 设观察点，值变化时自动暂停。
- [ ] 指令面板：在源码下方显示即将执行指令的语义说明（自然语言 + 等价 C 伪代码），从 `vm/i8086/isa/` 元数据生成。
- [ ] 诊断浮层：执行错误（越界、未实现指令、除零）以模态框展示，回车关闭并保留状态。
- [ ] 简易调用栈：根据 `call/ret` 维护逻辑调用栈，单独面板（按 `Tab` 可切到）。
- [ ] 主题/配色可在 `~/.config/vasm/config.toml` 中覆盖。

---

## M7 · v1 发布

- [ ] README：截屏、安装、教程示例、教材章节对照表。
- [ ] `cargo install --path .` 一键安装通过。
- [ ] `examples/` 按教材章节组织（`ch03_*.asm`、`ch05_loop.asm` …），至少覆盖教材每章 1~2 个程序。
- [ ] 性能基线：1e6 条指令的循环执行 < 1s（debug 模式下可放宽）。
- [ ] v0.1.0 tag + GitHub Release。

---

## v2 路线（x86-64，发布后启动）

- 在 `vm/x86_64/` 下新建独立实现；与 `vm/i8086/` 并列。
- 此时再回看 `vm/mod.rs`，按两套实现的真实差异引入最小抽象（很可能只需要一个 `trait Cpu` + `enum Arch`）。
- CLI 增加 `--arch i8086|x86_64`（默认 `i8086`，保持向后兼容）。
- AST 层考虑拆分语法方言：MASM-16 / NASM-64。
- 这一阶段写独立的 `docs/plan_v2.md`，本文档不再追加。

---

## 进度记录

| 里程碑 | 状态     | 完成日期 |
| ------ | -------- | -------- |
| M0     | ✅ 完成   | 2026-05-20 |
| M1     | ✅ 完成   | 2026-05-20 |
| M2     | 未开始   |          |
| M3     | 未开始   |          |
| M4     | 未开始   |          |
| M5     | 未开始   |          |
| M6     | 未开始   |          |
| M7     | 未开始   |          |
