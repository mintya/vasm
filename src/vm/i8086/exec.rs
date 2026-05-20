use thiserror::Error;

use crate::asm::ast::{Instruction, Program};
use crate::asm::diagnostics::Span;
use crate::vm::i8086::cpu::Cpu;
use crate::vm::i8086::isa;
use crate::vm::i8086::loader::{self, LoadError, LoadedProgram, SegmentLayout};
use crate::vm::i8086::memory::{MemError, Memory};

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum VmError {
    #[error("unsupported instruction `{mnemonic}`")]
    UnsupportedInstruction { mnemonic: String, span: Span },
    #[error("undefined symbol `{name}`")]
    UndefinedSymbol { name: String, span: Span },
    #[error("invalid operand: {reason}")]
    InvalidOperand { reason: String, span: Span },
    #[error("cannot move immediate into segment register")]
    SegRegImmediate { span: Span },
    #[error("divide by zero")]
    DivideByZero { span: Span },
    #[error("execution exceeded {max_steps} steps without halting")]
    StepLimitExceeded { max_steps: u64 },
    #[error("program has no executable entry point")]
    EntryRequired,
    #[error("ip 0x{ip:04X} does not map to any instruction in segment `{seg}`")]
    BadInstructionPointer { seg: String, ip: u16 },
    #[error("cs=0x{cs:04X} does not match any loaded segment")]
    UnknownCodeSegment { cs: u16 },
    #[error(transparent)]
    Mem(#[from] MemError),
    #[error(transparent)]
    Load(#[from] LoadError),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepOutcome {
    Stepped,
    Halted,
}

pub struct Vm {
    pub cpu: Cpu,
    pub mem: Memory,
    pub program: LoadedProgram,
    halted: bool,
}

impl Vm {
    pub fn boot(program: Program, mem_kb: u32) -> Result<Self, VmError> {
        let (loaded, memory) = loader::load(&program, mem_kb, loader::DEFAULT_START_PARAGRAPH)?;
        let mut cpu = Cpu::new();
        let (cs, ip) = loaded.entry.clone().ok_or(VmError::EntryRequired)?;
        cpu.cs = loaded.segments[&cs].base_paragraph;
        cpu.ip = ip;
        Ok(Self {
            cpu,
            mem: memory,
            program: loaded,
            halted: false,
        })
    }

    pub fn halted(&self) -> bool {
        self.halted
    }

    pub fn halt(&mut self) {
        self.halted = true;
    }

    /// 用于 isa 实现：跳转指令显式设置 ip。
    pub fn set_ip(&mut self, new_ip: u16) {
        self.cpu.ip = new_ip;
    }

    pub fn step(&mut self) -> Result<StepOutcome, VmError> {
        if self.halted {
            return Ok(StepOutcome::Halted);
        }

        let cs = self.cpu.cs;
        let ip = self.cpu.ip;
        let seg = self
            .find_segment_by_paragraph(cs)
            .ok_or(VmError::UnknownCodeSegment { cs })?
            .clone();

        // 找到当前 ip 对应的指令
        let slot = match seg.instructions.iter().find(|s| s.ip_offset == ip) {
            Some(s) => s.clone(),
            None => {
                // ip 没落在任何指令上 —— 通常是走出代码段末尾，做一次隐式 fall-off halt
                if seg
                    .instructions
                    .last()
                    .is_some_and(|s| ip >= s.ip_offset + s.size)
                {
                    self.halted = true;
                    return Ok(StepOutcome::Halted);
                }
                return Err(VmError::BadInstructionPointer {
                    seg: seg.name.clone(),
                    ip,
                });
            }
        };

        // 默认 advance ip；跳转指令可以在 dispatch 内覆盖。
        self.cpu.ip = ip + slot.size;

        isa::dispatch(self, &slot.instr, slot.span)?;

        if self.halted {
            Ok(StepOutcome::Halted)
        } else {
            Ok(StepOutcome::Stepped)
        }
    }

    pub fn run_until_halt(&mut self, max_steps: u64) -> Result<(), VmError> {
        for _ in 0..max_steps {
            if let StepOutcome::Halted = self.step()? {
                return Ok(());
            }
        }
        Err(VmError::StepLimitExceeded { max_steps })
    }

    pub fn current_instruction(&self) -> Option<&Instruction> {
        let seg = self.find_segment_by_paragraph(self.cpu.cs)?;
        seg.instructions
            .iter()
            .find(|s| s.ip_offset == self.cpu.ip)
            .map(|s| &s.instr)
    }

    pub fn segment_for(&self, name: &str) -> Option<&SegmentLayout> {
        self.program.segments.get(name)
    }

    fn find_segment_by_paragraph(&self, paragraph: u16) -> Option<&SegmentLayout> {
        self.program
            .segments
            .values()
            .find(|s| s.base_paragraph == paragraph)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::asm::parser::parse;

    fn boot(src: &str) -> Vm {
        let (prog, diags) = parse(src);
        assert!(diags.is_empty(), "parse diags: {diags:?}");
        Vm::boot(prog, 1024).expect("boot")
    }

    #[test]
    fn entry_cs_ip_initialized() {
        let vm = boot("code segment\nstart:\n  hlt\ncode ends\nend start\n");
        assert_eq!(vm.cpu.cs, loader::DEFAULT_START_PARAGRAPH);
        assert_eq!(vm.cpu.ip, 0);
    }

    #[test]
    fn hlt_halts_after_one_step() {
        let mut vm = boot("code segment\n  hlt\ncode ends\nend\n");
        assert_eq!(vm.step().unwrap(), StepOutcome::Halted);
        assert!(vm.halted());
    }

    #[test]
    fn falling_off_segment_halts_implicitly() {
        // nop without trailing hlt; ip moves past last instruction → fall-off halt
        let mut vm = boot("code segment\n  nop\ncode ends\nend\n");
        assert_eq!(vm.step().unwrap(), StepOutcome::Stepped);
        assert_eq!(vm.step().unwrap(), StepOutcome::Halted);
    }

    #[test]
    fn unsupported_instruction_returns_error() {
        // mul 在 M4 才实现，M2 应当报 UnsupportedInstruction
        let mut vm = boot("code segment\n  mul bx\n  hlt\ncode ends\nend\n");
        let err = vm.step().unwrap_err();
        assert!(matches!(err, VmError::UnsupportedInstruction { .. }));
    }

    #[test]
    fn step_limit_caught() {
        // nop loop won't actually loop yet; this just tests the limit triggers if many steps
        let mut vm = boot("code segment\n  nop\ncode ends\nend\n");
        // 不带 max_steps 限制会因 fall-off 在第 2 步停机，所以这里测一个紧凑上限
        let err = vm.run_until_halt(1).unwrap_err();
        assert!(matches!(err, VmError::StepLimitExceeded { .. }));
    }
}
