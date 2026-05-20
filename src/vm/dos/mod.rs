//! DOS `int 21h` stub。
//!
//! 教学只覆盖王爽教材正文与课后实验出现的功能号：01/02/09/0A/4C。
//! 其他 ah 一律返回 `UnsupportedDosFunc`，让用户在 TUI 中看到明确错误。

use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::Reg8;
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::memory::Memory;

pub fn int21(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let ah = vm.cpu.r8(Reg8::Ah);
    match ah {
        0x01 => func_01_read_char_with_echo(vm),
        0x02 => func_02_write_char(vm),
        0x09 => func_09_write_string(vm, span),
        0x0A => func_0a_buffered_input(vm, span),
        0x4C => {
            vm.halt();
            Ok(())
        }
        n => Err(VmError::UnsupportedDosFunc { ah: n, span }),
    }
}

/// 01h：读单字符（阻塞），写入 al，并回显到 Console。
fn func_01_read_char_with_echo(vm: &mut Vm) -> Result<(), VmError> {
    match vm.console.pop_input() {
        Some(b) => {
            vm.cpu.set_r8(Reg8::Al, b);
            vm.console.push_output(b);
            Ok(())
        }
        None => {
            vm.console.set_waiting(true);
            Ok(())
        }
    }
}

/// 02h：把 dl 作为字符输出。
fn func_02_write_char(vm: &mut Vm) -> Result<(), VmError> {
    let ch = vm.cpu.r8(Reg8::Dl);
    vm.console.push_output(ch);
    Ok(())
}

/// 09h：输出 `$` 结尾字符串。地址由 ds:dx 给出。
fn func_09_write_string(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let ds = vm.cpu.ds;
    let mut off = vm.cpu.dx;
    let mut budget = 4096usize;
    loop {
        if budget == 0 {
            return Err(VmError::InvalidOperand {
                reason: "int 21h ah=09h: string longer than 4096 bytes without '$'".into(),
                span,
            });
        }
        let phys = Memory::phys(ds, off);
        let b = vm.mem.read_u8(phys)?;
        if b == b'$' {
            return Ok(());
        }
        vm.console.push_output(b);
        off = off.wrapping_add(1);
        budget -= 1;
    }
}

/// 0Ah：缓冲键盘输入（DOS 行编辑代劳）。
/// 入口：ds:dx 指向缓冲区，缓冲区首字节 = max（含末尾回车，所以最多读 max-1 个字符）。
/// 出口：[ds:dx+1] = 实际字符数（不含回车），[ds:dx+2..] = 字符串 + 0x0D 收尾。
///
/// DOS 在这一调用里"代劳行编辑"——逐字节读 input 并立即回显，碰 0x08 时
/// 既缩短缓冲计数，也写 `08 20 08` 到屏幕做视觉擦除；碰 0x0D 时写 CRLF 退出。
fn func_0a_buffered_input(vm: &mut Vm, _span: Span) -> Result<(), VmError> {
    let ds = vm.cpu.ds;
    let dx = vm.cpu.dx;
    let max = vm.mem.read_u8(Memory::phys(ds, dx))?;
    let cap = max.saturating_sub(1);

    let mut count: u8 = 0;
    loop {
        match vm.console.pop_input() {
            None => {
                vm.console.set_waiting(true);
                return Ok(());
            }
            Some(b'\r') => {
                vm.mem
                    .write_u8(Memory::phys(ds, dx.wrapping_add(1)), count)?;
                vm.mem.write_u8(
                    Memory::phys(ds, dx.wrapping_add(2).wrapping_add(count as u16)),
                    0x0D,
                )?;
                // DOS 写 CRLF 让屏幕换行
                vm.console.push_output(0x0D);
                vm.console.push_output(0x0A);
                return Ok(());
            }
            Some(0x08) => {
                // Backspace：若缓冲有内容，缩短计数 + 视觉擦除（08 20 08）；
                // 缓冲为空时 DOS 行为是"哔"一声（教学场景静默忽略）。
                if count > 0 {
                    count -= 1;
                    vm.console.push_output(0x08);
                    vm.console.push_output(b' ');
                    vm.console.push_output(0x08);
                }
            }
            Some(b) => {
                if count < cap {
                    vm.mem.write_u8(
                        Memory::phys(ds, dx.wrapping_add(2).wrapping_add(count as u16)),
                        b,
                    )?;
                    count = count.saturating_add(1);
                    vm.console.push_output(b);
                }
                // 溢出时 DOS 也会"哔"，教学场景静默丢弃。
            }
        }
    }
}
