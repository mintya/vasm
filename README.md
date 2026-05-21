# VisualASM

> 在终端里直观看见 8086 汇编程序每一条指令的效果。

[![crates.io](https://img.shields.io/crates/v/vasm.svg)](https://crates.io/crates/vasm)
[![docs.rs](https://docs.rs/vasm/badge.svg)](https://docs.rs/vasm)
[![CI](https://github.com/mintya/vasm/actions/workflows/ci.yml/badge.svg)](https://github.com/mintya/vasm/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT_OR_Apache--2.0-blue.svg)](#license)

VisualASM 是一个跨平台的教学型 8086 实模式汇编 TUI 调试器，灵感来自 DOS 时代的 `debug.exe`：单步、断点、寄存器/段/标志位/栈/内存所见即所得，再加现代终端的体验（焦点切换、撤销、观察点、外部编辑器联动）。

适合场景：
- 学 / 教 16 位汇编时，需要"看见每条指令具体改了什么"
- 跟一段网上的 8086 代码片段，想跑起来看寄存器走向
- 在没有 DOSBox 的 macOS / Linux 上想要 `debug.exe` 替代品

## 截图

```
╭────────────────────────────────────── VisualASM · ch05_sum_loop.asm ───────────────────────────────────────╮
│ vasm  ch05_sum_loop.asm  ● Paused  cs:ip=1000:000C  #steps=8  #int=0  focus=Source  mode=CTRL              │
│┌Source [F1]────────────────────────────┐┌ Console [F2] ──────────────┐┌ Registers [F3] ───────────────────┐│
││    3 │ code segment                   ││█                           ││General  ax=0003  bx=0003  cx=0009 ││
││    4 │ start:                         ││                            ││         ah=00 al=03   bh=00 bl=03 ││
││    5 │     mov ax, 0                  ││                            │└───────────────────────────────────┘│
││    6 │     mov cx, 10                 ││                            │┌ Segments ─────────────────────────┐│
││    7 │     mov bx, 1                  ││────────────────────────────││cs=1000  (code)                    ││
││    8 │ sum_loop:                      ││>                           ││ds=0000  -                         ││
││    9 │     add ax, bx                 │└────────────────────────────┘│ss=1000  (stack)                   ││
││   10 │     inc bx                     │┌ Call Stack [F4] (0) ───────┐└───────────────────────────────────┘│
││ ▶ 11 │     loop sum_loop              ││(empty — call 后生成栈帧)    │┌Flags──────────────────────────────┐│
││   12 │     hlt                        ││                            ││CF· PF✓ AF· ZF· SF· TF· IF· DF· OF·││
│└───────────────────────────────────────┘└────────────────────────────┘└───────────────────────────────────┘│
│┌Memory  0000:0000─────────────────────────────────────────────────────────────────────────────────────────┐│
││0000  00 00 00 00 00 00 00 00  00 00 00 00 00 00 00 00  ................                                  ││
│└──────────────────────────────────────────────────────────────────────────────────────────────────────────┘│
│▶ loop sum_loop    ; cx-=1, cx!=0 则跳转                                                                     │
│[s]单步 [n]步过 [c]继续 [b]断点 [u]撤销 [w]watch [r]复位 [g]跳转 [Tab]焦点 [e]编辑 [q]退出                       │
╰────────────────────────────────────────────────────────────────────────────────────────────────────────────╯
```

## 安装

```bash
# 从 crates.io 一行装好
cargo install vasm

# 或者从源码（开发/最新功能）
git clone https://github.com/mintya/vasm
cd vasm
cargo install --path . --locked

# 装完任何目录都能跑
vasm examples/ch05_sum_loop.asm
```

最低 Rust 版本：1.85（edition 2024）。

## 快速开始

```bash
vasm examples/ch05_sum_loop.asm
```

启动后默认 **Paused** 在入口指令，等你按键。

- 按 `s` 单步一条指令，看右边寄存器变
- 按 `b` 在光标行设/取消断点
- 按 `c` 跑到 halt 或下一个断点
- 按 `q` 退出

## 键位

### 控制模式（默认）

| 键 | 作用 |
|----|------|
| `s` | 单步一条指令 |
| `n` | 步过（遇到 call 时跳过整个子过程） |
| `c` | 持续执行到 halt / 断点 / 等待输入 |
| `b` | 在光标所在行 toggle 断点 |
| `u` | 撤销上一步（回退寄存器/内存/Console 输出） |
| `U` | 撤销到上一个断点 |
| `w` | 添加观察点（寄存器名 / `seg:off` / 物理地址） |
| `W` | 清空所有观察点 |
| `r` | 复位 VM（保留断点） |
| `g` | goto：跳到 `seg:off` / 标签 / 物理地址 |
| `e` | 调起 `$EDITOR` 改源码，退出后重载 |
| `Tab` / `BackTab` | 切换面板焦点 |
| `F1`–`F4` | 直达 Source / Console / Registers / CallStack |
| `↑` `↓` `PgUp` `PgDn` | 移光标（Source 焦点）/ 滚动（Memory / CallStack） |
| `q` / `Ctrl+C` | 退出 |

### 输入模式（Console 焦点）

`Tab` / `F2` 切到 Console 后进入输入模式：按键直接写入 VM 输入缓冲（程序用 `int 21h ah=01h` 等读取）。

| 键 | 作用 |
|----|------|
| 可打印字符 / `Enter` / `Tab` / `Backspace` | 入 VM 输入缓冲 |
| `Esc` | 退回控制模式 |
| `PgUp` / `PgDn` | 滚动 Console 输出 |
| `Ctrl+C` | 强制退出 |

### Prompt / Error 模态

- Prompt（`g` 跳转、`w` 添加 watch 时出现）：`Enter` 提交，`Esc` 取消
- 错误浮层：`Enter` 或 `Esc` 关闭，VM 状态保留供检查

## 示例

`examples/` 按学习曲线组织，文件名前缀按从浅入深的主题编号：

| 文件 | 教学点 |
|------|--------|
| `ch01_smoke.asm` | 最小 nop 烟雾测试 |
| `ch02_first_mov.asm` | `mov` 立即数 + `add` + `inc` 寄存器运算 |
| `ch03_mem_access.asm` | `[bx]` 读字节，段寄存器赋值 |
| `ch04_first_program.asm` | `assume` / `segment-ends` / `end start` 完整骨架 |
| `ch05_sum_loop.asm` | `loop` 循环求和 1+2+…+10 |
| `ch06_multi_segment.asm` | 多段程序 + `ds`/`ss` 赋值 + push/pop |
| `ch07_addressing.asm` | `[bx+si]` 复合寻址访问矩阵元素 |
| `ch08_data_dup.asm` | `db 'string'` + `loop` + 按位运算（大小写转换） |
| `ch10_call_ret.asm` | `call` / `ret` 子过程调用 + 栈对称 |
| `ch11_cmp_jne.asm` | `cmp` + `jne` 条件跳转倒数累加 |
| `ch12_int_div_zero.asm` | 除零内中断触发 `DivideByZero` |
| `ch13_hello.asm` | DOS `int 21h ah=09h` 输出 `$` 结尾字符串 |
| `ch13_echo.asm` | DOS `int 21h ah=01h` 阻塞读字符 + 回显 |
| `ch13_bios_video.asm` | BIOS `int 10h ah=09h` 重复输出字符 |
| `ch14_port_in.asm` | `in al, 60h` 读端口 |
| `ch16_jump_table.asm` | `call word ptr ds:[...]` 间接跳转（函数指针表） |
| `ch17_disk_boot.asm` | `int 13h ah=02h` 读软盘启动扇区（需 `--disk`） |

跑磁盘示例：

```bash
# 准备一张 1.44MB 虚拟软盘
dd if=/dev/zero of=floppy.img bs=512 count=2880
vasm --disk floppy.img examples/ch17_disk_boot.asm
```

## 配置

VisualASM 启动时尝试读取 `~/.config/vasm/config.toml`，所有字段可选，未设值使用默认。

```toml
[theme]
# 命名色：black / red / green / yellow / blue / magenta / cyan /
#         gray / darkgray / lightred / lightgreen / lightyellow / lightblue /
#         lightmagenta / lightcyan / white / reset
# 或 #RRGGBB hex
border          = "cyan"
border_focused  = "yellow"
console_output  = "green"
console_echo    = "yellow"
register_value  = "white"
register_name   = "gray"
flag_set        = "green"
flag_clear      = "darkgray"
status_paused   = "yellow"
status_halted   = "green"
status_error    = "red"
status_waiting  = "magenta"
source_keyword  = "yellow"
source_register = "cyan"
source_number   = "green"
source_pc       = "yellow"
muted           = "darkgray"
```

完整字段清单见 `src/theme.rs`。

## CLI 参数

```
vasm <FILE> [OPTIONS]

参数：
  <FILE>              .asm 源文件路径

可选：
  --log <PATH>        把日志写到指定文件（默认不开）
  --mem-kb <KB>       模拟内存大小（KiB，默认 1024）
  --encoding <ENC>    Console 字节流编码：utf8 / gbk（默认 gbk）
  --disk <PATH>       挂载虚拟软盘镜像（int 13h 用）
  --max-steps <N>     headless --run 模式下指令上限（默认 1_000_000）
```

## 体系结构

```
src/
├── asm/         # 手写 lexer + 递归下降 parser → AST
├── vm/i8086/    # CPU + Memory + ISA dispatch + BIOS/DOS stub
│   ├── isa/     # 指令分组：arith / control / data_move / logic / shift / stack / intr / io
│   ├── memory.rs   # 段式寻址，支持 undo 录写
│   └── exec.rs     # step / step_with_snapshot（undo 用）
├── ui/          # ratatui 渲染层，纯只读
│   └── panes/   # source / console / registers / memory / stack / flags / segments / call_stack / explain / status / keymap / diagnostic / prompt
├── app/         # 状态机 + keymap + event loop
└── theme.rs     # 可外配主题
```

主要设计决策见 [`docs/conventions.md`](docs/conventions.md) 与 [`docs/plan.md`](docs/plan.md)。

## 当前局限

- 仅支持 8086 实模式（16 位）；x86-64 走独立 v2 路线
- 不实现外中断异步注入（IRQ 事件）
- 汇编前端不实现宏 / `include` / 结构体 / `proc/endp` / 条件汇编
- DOS / BIOS 调用按按需 stub，不追求完整覆盖

## License

MIT OR Apache-2.0 —— 下游可任选其一。详见 [`LICENSE-MIT`](LICENSE-MIT) 与 [`LICENSE-APACHE`](LICENSE-APACHE)。
