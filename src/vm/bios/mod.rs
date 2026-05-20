//! BIOS `int 10h` / `int 16h` stub。
//!
//! `int 10h`：00（设显示模式，仅记录）、02（设光标位置，仅记录）、09/0A（按 cx 写字符）、13（写字符串 es:bp）。
//! `int 16h`：00（阻塞读键）、01（非阻塞查键）。
//! 教学只覆盖王爽教材正文用到的功能号，其他报 `UnsupportedBiosFunc`。

use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::{Reg8, Reg16};
use crate::vm::i8086::exec::{Vm, VmError};
use crate::vm::i8086::memory::Memory;

pub fn int10(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let ah = vm.cpu.r8(Reg8::Ah);
    match ah {
        0x00 => {
            let mode = vm.cpu.r8(Reg8::Al);
            vm.console.set_display_mode(mode);
            Ok(())
        }
        0x02 => {
            let row = vm.cpu.r8(Reg8::Dh);
            let col = vm.cpu.r8(Reg8::Dl);
            vm.console.set_cursor(row, col);
            Ok(())
        }
        0x09 | 0x0A => {
            let ch = vm.cpu.r8(Reg8::Al);
            let count = vm.cpu.cx;
            for _ in 0..count {
                vm.console.push_output(ch);
            }
            Ok(())
        }
        0x13 => {
            // es:bp 起的 cx 个字节。教学忽略 al(写模式) 与 bh/bl(属性)。
            let es = vm.cpu.es;
            let mut off = vm.cpu.r16(Reg16::Bp);
            for _ in 0..vm.cpu.cx {
                let b = vm.mem.read_u8(Memory::phys(es, off))?;
                vm.console.push_output(b);
                off = off.wrapping_add(1);
            }
            Ok(())
        }
        n => Err(VmError::UnsupportedBiosFunc {
            int_num: 0x10,
            ah: n,
            span,
        }),
    }
}

pub fn int16(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let ah = vm.cpu.r8(Reg8::Ah);
    match ah {
        0x00 => {
            // 阻塞读键：al = ASCII，ah = 扫描码（教学填 0）。
            match vm.console.pop_input() {
                Some(b) => {
                    vm.cpu.ax = b as u16;
                    Ok(())
                }
                None => {
                    vm.console.set_waiting(true);
                    Ok(())
                }
            }
        }
        0x01 => {
            // 非阻塞查键：ZF=0 表示有键（且 ah/al 填入），ZF=1 表示无键。不消费缓冲。
            match vm.console.peek_input() {
                Some(b) => {
                    vm.cpu.ax = b as u16;
                    vm.cpu.flags.zf = false;
                }
                None => {
                    vm.cpu.flags.zf = true;
                }
            }
            Ok(())
        }
        n => Err(VmError::UnsupportedBiosFunc {
            int_num: 0x16,
            ah: n,
            span,
        }),
    }
}
