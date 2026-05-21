# Changelog

本文件记录 VisualASM 各版本的变更。格式参考 [Keep a Changelog](https://keepachangelog.com/)，版本号遵循 [SemVer](https://semver.org/)。

## [0.1.0] - 2026-05-21

首次发布。完成了从汇编前端到 TUI 交互调试的完整链路。

### Added

#### 汇编前端
- 手写 lexer + 递归下降 parser，支持 MASM 风格 Intel 语法
- 伪指令：`segment/ends`、`assume`、`db/dw/dd`、`dup`、`offset`、`end <label>`、`org`
- 寻址形式：`[bx]`、`[bx+si]`、`[bx+idata]`、`[bx+si+idata]`、`ds:[...]`、`word/dword/byte ptr`
- 立即数：十进制、`H` 后缀十六进制、`B` 后缀二进制、字符常量、字符串

#### VM 内核（8086 实模式）
- CPU：完整 8 个 16 位通用寄存器 + 8 位别名（ah/al/...）、4 个段寄存器、ip、9 位 flags
- Memory：1 MiB 线性数组，`phys(seg, off) = seg << 4 + off`
- 段加载器：按 `segment` 顺序铺到内存，`end start` 决定入口
- 指令族：数据传送 / 栈 / 算术 / 逻辑 / 移位 / 控制流 / 条件跳转 / 中断 / I/O
  - 间接跳转：`jmp word ptr ds:[bx]`、`call word ptr ds:[bx]`、`jmp reg`
  - 完整条件跳转族（18+ 助记符走统一 jcc）

#### 中断与 I/O
- DOS `int 21h`：ah=01h（阻塞读）/ 02h（输出 dl）/ 09h（输出 `$` 串）/ 0Ah（缓冲输入）/ 4Ch（退出）
- BIOS `int 10h`：ah=02h（光标）/ 09h（重复字符）/ 13h（写串）
- BIOS `int 13h`：ah=00h/02h/03h（磁盘重置/读/写扇区，CHS↔LBA 换算）
- BIOS `int 16h`：ah=00h/01h/02h/11h（阻塞读 / 非阻塞查 / 状态字节 / 扩展查键）
- 用户中断：完整 IVT 寻址 + iret 还原
- `in al, 60h` 接键盘扫描码、`cli`/`sti` 维护 IF
- ConsoleIo：output 缓冲 + input 队列 + waiting 协议
- 编码：CLI flag 选 utf8/gbk；VM 内部一律按字节存

#### TUI（基于 ratatui + crossterm）
- 13 个面板：Source / Console / Registers / Segments / Flags / Stack / Memory / Call Stack / Explain / Status / Keymap / Prompt / Diagnostic
- 跨平台焦点环：Source / Console / Registers / Memory / CallStack（Tab 循环 + F1–F4 直达）
- 源码面板：语法高亮、PC `▶` 标记、断点 `●` 标记、cursor 行整行高亮
- Console 面板：DOS 终端语义（\r/\n/\b/\t），echo 字符 caret 显示，按 PgUp/PgDn 滚动
- Registers / Segments / Flags：实时反映 cpu 状态，三档色阶分区
- Memory：可定位段:偏移，16 字节/行 hex+ascii
- Call Stack：根据 call/ret 维护逻辑栈
- Explain：当前指令的元数据驱动注释 + 操作数代入

#### 交互调试
- 单步 `s` / 步过 `n` / 持续 `c` / 复位 `r`
- 断点 `b`（任意行 toggle）
- goto `g`（seg:off / 标签 / 物理地址）
- 撤销 `u` / 撤销到上个断点 `U`（cpu + memory + console 三层快照，上限 1024 步）
- 观察点 `w` / 清空 `W`（寄存器 / seg:off / 物理地址，命中变化自动 Paused）
- 错误模态浮层（VmError 弹诊断窗口，Enter 关闭保留 VM 状态）
- 外部编辑器 `e`（$EDITOR / $VISUAL 回退链，退出后自动重载）

#### 主题与配置
- `~/.config/vasm/config.toml` 覆盖 24 项颜色字段
- 支持命名色 + `#RRGGBB` hex
- 解析失败回退默认（DOS 绿风格）

#### 示例程序
- 17 个 `examples/chXX_*.asm` 按学习曲线编号，覆盖 mov/算术/寻址/loop/多段/call/cmp/中断/磁盘 等核心主题
- 含教学注释 + 终态断言（与 `tests/vm_run.rs` 联动）

#### 测试
- 224 项测试：lib 单测 112 / parser 快照 10 / ui_render 44 / vm_run 58
- 全部 platform-agnostic，可在 Linux / macOS / Windows 跑

### Known Limitations
- 外中断异步注入（IRQ 事件）未实现
- 无性能基线（debug 模式下教学场景够用）
- 仅 16 位 8086 实模式；x86-64 走独立 v2 路线
- 汇编前端不支持宏 / `include` / 结构体 / `proc/endp` / 条件汇编

[0.1.0]: https://github.com/mintya/vasm/releases/tag/v0.1.0
