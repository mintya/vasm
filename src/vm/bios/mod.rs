//! BIOS `int 10h` / `int 13h` / `int 16h` stub。
//!
//! `int 10h`：00（设显示模式，仅记录）、02（设光标位置，仅记录）、09/0A（按 cx 写字符）、13（写字符串 es:bp）。
//! `int 13h`：00（重置磁盘 no-op）、02（读扇区）、03（写扇区）——按 1.44MB 软盘 CHS 几何。
//! `int 16h`：00（阻塞读键）、01（非阻塞查键）、02（取键盘状态 stub）、11（扩展非阻塞查键）。
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
        0x00 | 0x10 => {
            // 阻塞读键。10h = 扩展键码版本，教学场景行为同 00h。
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
        0x01 | 0x11 => {
            // 非阻塞查键：ZF=0 表示有键（且 ah/al 填入），ZF=1 表示无键。不消费缓冲。
            // 11h = 扩展键码版本，教学场景行为同 01h。
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
        0x02 | 0x12 => {
            // 取键盘状态字节（Shift/Ctrl/Alt 等修饰键）。教学场景不模拟修饰键，返 0。
            vm.cpu.set_r8(Reg8::Al, 0);
            Ok(())
        }
        n => Err(VmError::UnsupportedBiosFunc {
            int_num: 0x16,
            ah: n,
            span,
        }),
    }
}

// ---- int 13h 磁盘 -------------------------------------------------------

/// 1.44MB 软盘几何：2 磁头 × 80 柱面 × 18 扇区/磁道 × 512 字节。
const HEADS: u32 = 2;
const SECTORS_PER_TRACK: u32 = 18;
const SECTOR_SIZE: u32 = 512;

pub fn int13(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let ah = vm.cpu.r8(Reg8::Ah);
    match ah {
        0x00 => {
            // 重置磁盘控制器：no-op；返 AH=0、CF=0 表成功
            vm.cpu.set_r8(Reg8::Ah, 0);
            vm.cpu.flags.cf = false;
            Ok(())
        }
        0x02 => disk_read(vm, span),
        0x03 => disk_write(vm, span),
        n => Err(VmError::UnsupportedBiosFunc {
            int_num: 0x13,
            ah: n,
            span,
        }),
    }
}

/// ah=02h 读扇区：al=扇区数, ch=柱面, cl=扇区号(1-based), dh=磁头, dl=驱动器,
/// es:bx=缓冲。成功 AH=0/CF=0，失败 AH=错误码/CF=1。
fn disk_read(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let (lba, count) = match resolve_chs(vm, span) {
        Ok(v) => v,
        Err(e) => {
            disk_fail(vm, 0x04); // sector not found
            return Err(e);
        }
    };
    let disk = vm.disk.as_ref().ok_or_else(|| VmError::DiskIo {
        reason: "no disk attached (use --disk)".into(),
        span,
    })?;
    let start = lba as usize * SECTOR_SIZE as usize;
    let end = start + count as usize * SECTOR_SIZE as usize;
    if end > disk.len() {
        disk_fail(vm, 0x04);
        return Err(VmError::DiskIo {
            reason: format!("read past disk end: lba={lba} count={count}"),
            span,
        });
    }
    let bytes = disk[start..end].to_vec();
    let es = vm.cpu.es;
    let bx = vm.cpu.bx;
    for (i, b) in bytes.iter().enumerate() {
        let off = bx.wrapping_add(i as u16);
        vm.mem.write_u8(Memory::phys(es, off), *b)?;
    }
    vm.cpu.set_r8(Reg8::Ah, 0);
    vm.cpu.flags.cf = false;
    Ok(())
}

/// ah=03h 写扇区：参数同读，方向相反。
fn disk_write(vm: &mut Vm, span: Span) -> Result<(), VmError> {
    let (lba, count) = match resolve_chs(vm, span) {
        Ok(v) => v,
        Err(e) => {
            disk_fail(vm, 0x04);
            return Err(e);
        }
    };
    if vm.disk.is_none() {
        return Err(VmError::DiskIo {
            reason: "no disk attached (use --disk)".into(),
            span,
        });
    }
    let start = lba as usize * SECTOR_SIZE as usize;
    let total_bytes = count as usize * SECTOR_SIZE as usize;
    let disk_len = vm.disk.as_ref().map(|d| d.len()).unwrap_or(0);
    if start + total_bytes > disk_len {
        disk_fail(vm, 0x04);
        return Err(VmError::DiskIo {
            reason: format!("write past disk end: lba={lba} count={count}"),
            span,
        });
    }
    let es = vm.cpu.es;
    let bx = vm.cpu.bx;
    let mut buf = Vec::with_capacity(total_bytes);
    for i in 0..total_bytes {
        let off = bx.wrapping_add(i as u16);
        buf.push(vm.mem.read_u8(Memory::phys(es, off))?);
    }
    vm.disk.as_mut().unwrap()[start..start + total_bytes].copy_from_slice(&buf);
    vm.cpu.set_r8(Reg8::Ah, 0);
    vm.cpu.flags.cf = false;
    Ok(())
}

/// CHS → LBA 并校验。返回 (lba, sector_count)。
fn resolve_chs(vm: &Vm, span: Span) -> Result<(u32, u8), VmError> {
    let count = vm.cpu.r8(Reg8::Al);
    let cyl = vm.cpu.r8(Reg8::Ch) as u32;
    let sect = vm.cpu.r8(Reg8::Cl) as u32;
    let head = vm.cpu.r8(Reg8::Dh) as u32;
    if sect == 0 || sect > SECTORS_PER_TRACK {
        return Err(VmError::DiskIo {
            reason: format!("invalid sector number {sect} (must be 1..={SECTORS_PER_TRACK})"),
            span,
        });
    }
    if head >= HEADS {
        return Err(VmError::DiskIo {
            reason: format!("invalid head {head} (must be 0..{HEADS})"),
            span,
        });
    }
    let lba = (cyl * HEADS + head) * SECTORS_PER_TRACK + (sect - 1);
    Ok((lba, count))
}

fn disk_fail(vm: &mut Vm, code: u8) {
    vm.cpu.set_r8(Reg8::Ah, code);
    vm.cpu.flags.cf = true;
}
