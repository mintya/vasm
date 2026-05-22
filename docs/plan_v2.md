# VisualASM v2 路线（x86-64 长模式）

本文档是 VisualASM v2 的分阶段实现路线图，承接 [`plan.md`](./plan.md) 末尾的 "v2 路线" 占位。
约定见 [`conventions.md`](./conventions.md)。

## 总览

v1（M0–M7）已发布完整的 8086 实模式 TUI 调试器，对齐 90 年代教学情境。
v2 把 ISA 与教学情境推到**现代 Linux 用户态汇编**：x86-64 长模式、NASM intel 语法、System V ABI 的 `syscall` 子集。

两套架构 **并列** 于 `vm/` 下，互不替代。新用户跑 v1 不受影响。

```
v1 (M0–M7) ✅ ─────── 8086 实模式 · MASM-16 · int 21h
                            ↓
                   vm/mod.rs 抽象层
                  （trait Cpu + enum Arch）
                            ↓
N0 架构抽象  →  N1 NASM 前端  →  N2 x86-64 VM 核心
                                       ↓
N5 v0.2.0 发布  ←  N4 TUI 适配 64-bit  ←  N3 Linux syscall ABI
```

目标产出：`vasm --arch x86_64 hello.nasm` 启动 TUI，左侧 NASM 源码，右侧 16 个 GP 寄存器、RFLAGS、64-bit 内存、调用栈；syscall write/read/exit/brk 子集可用；与 v1 共用同一个二进制、同一套 pane。

---

## 设计原则

1. **零回归**：`vm/i8086/` 与所有 v1 测试一行不动；每个 Stage 提交前 `cargo test` 全绿。
2. **两份独立前端**：MASM-16（v1）与 NASM-64（v2）共享 AST 通用部分（Instruction / Operand / Expr），方言差异用 enum 标识；各自一套 lexer/parser，互不影响。
3. **教学优先**：Linux 用户态 ABI 子集——`write` / `read` / `exit` / `brk` 四个 syscall，不做 paging / MMU / SMP / SSE。教学要点是"用 syscall 与内核交互"这件事本身。
4. **TUI 不分叉**：同一套 `ratatui` pane 通过 `Arch` 枚举内部分支适配 16-bit 与 64-bit 显示。不引入第二套 UI 代码。
5. **CLI 默认 v1**：`vasm file.asm` 等价于 `vasm --arch i8086 file.asm`，向后兼容是硬约束。

---

## N0 · 架构抽象 + scaffolding

**目标**：v1 代码逻辑完全保留，引入最小抽象让 v2 能并入。

- [ ] `src/vm/mod.rs`：加 `pub enum Arch { I8086, X86_64 }` + `pub trait Cpu` 最小接口
  - `fn step(&mut self) -> Result<StepOutcome, VmError>`
  - `fn halted(&self) -> bool`
  - `fn current_ip(&self) -> u64`（v1 截断到 u16）
  - 寄存器 dump 接口（供 TUI 通过 enum 分发渲染）
- [ ] `vm::i8086::exec::Vm` 实现 `Cpu` trait（保留所有原有方法，只增不改）
- [ ] `src/vm/x86_64/mod.rs`：占位空模块 + `TODO: N2`
- [ ] `src/cli.rs`：
  - `--arch <i8086|x86_64>` flag，默认 `i8086`
  - `--syntax <masm|nasm>` flag，默认按源文件启发式（`section .text` → nasm；`segment` → masm）
- [ ] `src/asm/` 拆 `masm/` 子模块：所有现有 lexer/parser/ast 文件迁移到 `src/asm/masm/`，原 `parse()` 改成 dispatch 入口（按 syntax flag 派发）
- [ ] `src/asm/ast.rs` 拆成 `common.rs`（共享 `Instruction` / `Operand` / `Expr`）+ `masm.rs`（`Segment` / `AssumeBinding`）

**不实现**：任何 x86-64 功能；NASM 解析的实际逻辑。

**验收**：v1 所有 219 项测试不变，`vasm --arch i8086 examples/ch05_sum_loop.asm` 行为完全等价于 N0 之前的 `vasm examples/ch05_sum_loop.asm`。

---

## N1 · NASM-64 前端

**目标**：能解析典型 Linux NASM 程序到 AST，不执行。

- [ ] `src/asm/nasm/lexer.rs`：NASM 关键字
  - 段：`section .text` / `section .data` / `section .bss`
  - 全局符号：`global` / `extern`
  - 数据：`db / dw / dd / dq` + `times N` + `equ`
  - 大小前缀：`byte` / `word` / `dword` / `qword`
- [ ] REX 寄存器名扩展：`rax..r15` / `eax..r15d` / `ax..r15w` / `al..r15b` / `spl/bpl/sil/dil`
- [ ] `src/asm/nasm/parser.rs`：递归下降，复用 v1 expr/operand 风格
- [ ] AST 扩展：复用共享 `Instruction` / `Operand` / `Expr`；新增 `SectionDirective`（NASM `.text` / `.data` / `.bss`）取代 `Segment`；`Mem` 加 `scale` 字段支持 SIB
- [ ] 寻址：`qword [rbx + rcx*8 + 0x10]` 完整 SIB 形式（base + index*scale + disp）
- [ ] `tests/nasm_snapshot.rs`：5~7 条 insta 快照覆盖 hello / loop / call / syscall

**不实现**：VM 执行；宏；`%include`；条件汇编。

**验收**：`cargo test --test nasm_snapshot` 全绿；NASM AST dump 与手写期望一致。

---

## N2 · x86-64 VM 核心

**目标**：能执行不需要 syscall 的纯计算 NASM 程序到 halt。

- [ ] `src/vm/x86_64/cpu.rs`：
  - 16 个 64-bit GP 寄存器 + 各级别名访问器（read/write 8/16/32/64）
  - rip / rsp / rbp
  - RFLAGS（复用 v1 `Flags` + 加 ID/VIP/VIF 等位作为 stub，渲染时只展示常用 9 位）
- [ ] `src/vm/x86_64/memory.rs`：
  - flat 64-bit 虚拟地址空间，模拟 1 GiB 物理上限
  - 代码段默认加载到 `0x0000_0000_0040_0000`（ELF 默认 base）
  - 栈从 `0x0000_7FFF_FFFF_E000` 向下生长
  - 不分页；越界返 `MemError::OutOfBounds`
- [ ] `src/vm/x86_64/exec.rs`：dispatcher 与 v1 同结构（step / step_with_snapshot / run_until_halt）
- [ ] `src/vm/x86_64/isa/`：每族指令一文件，最小集
  - data_move：`mov` / `lea`（lea 是 64-bit 高频指令，单独实现）
  - stack：`push` / `pop`（64-bit 操作数；rsp ±= 8）
  - arith：`add` / `sub` / `mul` / `imul` / `div` / `idiv` / `neg` / `cmp` / `inc` / `dec`
  - logic：`and` / `or` / `xor` / `not` / `test`
  - shift：`shl` / `shr` / `sar` / `sal`
  - control：`jmp` / `jcc`（同 v1 18+ 条件跳转族）/ `call` / `ret` / `loop` / `jrcxz`
  - misc：`hlt`（教学终止）

**不实现**：`syscall`（留 N3）；REP 前缀；SSE/AVX；privileged 指令。

**验收**：`tests/x86_64_run.rs` 加 5~7 条：mov 立即数 / add 链 / loop 求和 / call+ret 栈对称 / lea 算地址 / push+pop / cmp+jne 循环。

---

## N3 · Linux syscall ABI

**目标**：能跑 hello-world via `syscall write`。

- [ ] `src/vm/x86_64/isa/syscall.rs`：`syscall` 指令
  - 保存 rip → rcx，保存 rflags → r11（按真实硬件语义）
  - dispatch 到下面的 syscall 表
- [ ] `src/vm/x86_64/sys.rs`：System V ABI syscall dispatcher，按 `rax` 分号：
  - `0` read(fd=rdi, buf=rsi, count=rdx) — 从 `vm.console.input` 读，fd≠0 错；缓冲不足返 `WaitingForInput`
  - `1` write(fd=rdi, buf=rsi, count=rdx) — fd=1/2 写到 `vm.console`，其他丢弃
  - `60` exit(code=rdi) — `vm.halt()`；rdi 落到状态栏显示
  - `12` brk(addr=rdi) — 模拟堆顶；返新 brk 值到 rax
  - 其他 → `VmError::UnsupportedSyscall { num: rax, span }`
- [ ] 参数寄存器顺序按 System V：`rdi, rsi, rdx, r10, r8, r9`（r10 因为 syscall 不能用 rcx）
- [ ] 返回值：rax；rcx/r11 在 syscall 内被破坏（与真实硬件一致）

**验收**：`tests/x86_64_run.rs` 加 hello-world syscall write + exit(0) 测试；`vm.console.output()` 含 "hello, world\n"。

---

## N4 · TUI 适配 64-bit

**目标**：现有 ratatui pane 正确显示 x86-64 状态，无需新增 pane 文件。

- [ ] `src/ui/panes/registers.rs`：按 `Arch` enum 分支
  - i8086 路径不变
  - x86_64 路径：16 个 GP 寄存器分两列布局（左 rax..r7，右 r8..r15），每行一个 64-bit 值；下方分一行 rip / rsp / rbp 高亮
- [ ] `src/ui/panes/segments.rs`：x86_64 下隐藏（长模式段寄存器无用）或改成显示 fs/gs base
- [ ] `src/ui/panes/memory.rs`：地址列从 4-hex 扩到 16-hex；保持 16 字节/行
- [ ] `src/ui/panes/flags.rs`：x86_64 显示完整 RFLAGS 低 16 位
- [ ] `src/ui/panes/source.rs`：lexer dialect-aware 高亮——NASM 关键字 `section / global / syscall / qword / dq / equ`
- [ ] `src/ui/panes/explain.rs`：x86_64 走另一份 `vm/x86_64/isa/doc.rs`（仿 v1 `vm/i8086/isa/doc.rs`），InsnDoc 表覆盖 30+ 条

**复用**：所有 pane 走 `app.theme()` 取色，不引入新主题字段；用户在 `~/.config/vasm/config.toml` 仍可统一覆盖。

**验收**：`tests/ui_render.rs` 加 5~6 条 x86_64 渲染快照（registers / memory 64-bit / source NASM 高亮 / explain x64 doc）。

---

## N5 · 教学示例 + 文档 + v0.2.0 发布

**目标**：用户可以一键安装 + 跑通完整 demo。

- [ ] 新建 `examples/x64_*.asm` 7 个：
  - `x64_hello.asm` — syscall write 输出 hello-world
  - `x64_exit.asm` — 最小 exit(42)
  - `x64_loop.asm` — 用 `loop` / `dec rcx + jnz` 数 1..10
  - `x64_call.asm` — 子函数 + System V ABI 传参（rdi）
  - `x64_echo.asm` — syscall read + write 单字符回声
  - `x64_lea.asm` — `lea` 计算数组下标，演示无内存访问的地址运算
  - `x64_brk.asm` — brk 扩堆 + 写入 + 读回
- [ ] `tests/x86_64_run.rs` 末尾每个 fixture 一条 smoke
- [ ] `README.md`：加 "v2 / x86-64" 章节
  - `vasm --arch x86_64 examples/x64_hello.asm` 用法
  - 示例表（7 个新文件 + 教学点）
  - 与 v1 的对比表格（ISA / ABI / 语法）
- [ ] `CHANGELOG.md`：`[0.2.0] - YYYY-MM-DD`，Added: x86-64 long mode (NASM syntax, Linux syscall ABI subset)
- [ ] `Cargo.toml` 版本 `0.1.0 → 0.2.0`
- [ ] 本文档自身：勾完 N0–N5 + 进度记录表

**验收**：
- `cargo install --path . --locked` 通过
- `vasm --arch x86_64 examples/x64_hello.asm` TUI 启动，单步到 `syscall` 后 console 显示 "hello, world\n"
- `cargo test` 全绿（v1 219 项 + v2 新增 ~20 项）
- `cargo publish --dry-run` 通过

---

## v2 路线图节奏建议

参考 v1 经验，每个 Mxx 提交后停下等用户确认。v2 沿用同模式：

| 阶段 | 大致工作量 | 是否高风险 |
| ---- | ---------- | ---------- |
| N0   | 中（重构）| 是（要保 v1 测试不动）|
| N1   | 大（新前端）| 否 |
| N2   | 大（新 ISA）| 否 |
| N3   | 小（4 个 syscall）| 否 |
| N4   | 中（pane 双路径）| 是（snapshot 易碎）|
| N5   | 小（文档+示例）| 否 |

N0 是最重要的关口——抽象一旦设错就要重做。建议 N0 前先单独跑一次抽象草图 review。

---

## 不在 v2 范围

明确列出来避免 scope creep：

- **32-bit 保护模式 / IA-32**——跳过，v1 已经覆盖 16-bit 真模式
- **Windows x64 ABI / macOS Mach syscall**——后续 v3 视情况
- **GAS AT&T 语法**——教学反直觉，不做（`movq %rbx, %rax` 顺序反人类）
- **SSE / AVX / AVX-512 / TSX**——SIMD/事务指令族
- **Paging / MMU / TLB / 多核 / 抢占调度**——VM 仍是单 CPU 顺序执行
- **真实 ELF 加载**——v2 只跑单文件 .asm，不读 .o；不实现链接器
- **C 互操作 / FFI**——纯汇编教学
- **性能基线**——v1 已明确去掉，v2 继承

---

## 验收对齐 plan.md v2 占位

[`plan.md`](./plan.md) 末尾原有 5 行 v2 路线占位条目，本文档逐条落地如下：

| plan.md 占位条                                | plan_v2 落地处                                  |
| --------------------------------------------- | ----------------------------------------------- |
| `vm/x86_64/` 下新建独立实现                   | [N2](#n2--x86-64-vm-核心) + [N3](#n3--linux-syscall-abi) |
| `vm/mod.rs` 引入最小抽象                      | [N0](#n0--架构抽象--scaffolding)（`trait Cpu` + `enum Arch`） |
| CLI 增加 `--arch i8086\|x86_64`               | [N0](#n0--架构抽象--scaffolding)               |
| AST 拆语法方言（MASM-16 / NASM-64）           | [N0](#n0--架构抽象--scaffolding)（scaffolding） + [N1](#n1--nasm-64-前端)（NASM 实现） |
| 独立 `docs/plan_v2.md`                        | 本文档本身                                      |

---

## v3+ 远景（不承诺，仅占位）

- Windows x64 ABI + `WriteConsole` runtime stub
- 32-bit 保护模式作为 16 ↔ 64 的过渡章节
- ELF object loading（多文件链接）
- 简易 GDB 协议 server，让 VS Code/LLDB 连进来

每一项都要等到 v2 v0.2.0 稳定后再开新文档讨论。

---

## 进度记录

| 里程碑 | 状态     | 完成日期 |
| ------ | -------- | -------- |
| N0     | 未开始   |          |
| N1     | 未开始   |          |
| N2     | 未开始   |          |
| N3     | 未开始   |          |
| N4     | 未开始   |          |
| N5     | 未开始   |          |
