//! The CPU simulator: a step-able RV32IM core.
//!
//! Every `step()` is a real fetch → decode → execute cycle. Decode operates on
//! the actual bytes in memory - not a cached instruction list - so the machine
//! code panel genuinely drives execution.

use crate::assembler::decode;
use crate::ir::{Instr, SYSCALL_EXIT, SYSCALL_PRINT_INT};
use serde::Serialize;

pub const MEM_SIZE: usize = 64 * 1024; // 64 KiB

#[derive(Debug, Clone)]
pub struct Cpu {
    /// x0..x31. x0 is hardwired to zero (writes are discarded).
    pub regs: [i32; 32],
    pub pc: u32,
    pub mem: Vec<u8>,
    pub halted: bool,
}

/// What changed during one cycle - the lean delta shipped across the WASM
/// boundary instead of the whole CPU state.
#[derive(Debug, Clone, Serialize)]
pub struct StepDelta {
    pub pc_before: u32,
    pub pc_after: u32,
    /// Index into the program's instruction list (`pc_before / 4`), if the PC
    /// was inside the code section.
    pub instr_index: u32,
    pub reg_write: Option<RegWrite>,
    pub mem_write: Option<MemWrite>,
    /// Text emitted by a print ecall this cycle.
    pub output: Option<String>,
    pub halted: bool,
    /// Set when execution faulted (bad fetch / undecodable word).
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RegWrite {
    pub reg: u8,
    pub old: i32,
    pub new: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemWrite {
    pub addr: u32,
    pub old: i32,
    pub new: i32,
}

impl Cpu {
    /// A fresh CPU with `program` bytes loaded at address 0.
    pub fn new(program: &[u8]) -> Cpu {
        let mut mem = vec![0u8; MEM_SIZE];
        mem[..program.len()].copy_from_slice(program);
        Cpu {
            regs: [0; 32],
            pc: 0,
            mem,
            halted: false,
        }
    }

    pub fn read_word(&self, addr: u32) -> Option<i32> {
        let a = addr as usize;
        if a + 4 > self.mem.len() {
            return None;
        }
        Some(i32::from_le_bytes([
            self.mem[a],
            self.mem[a + 1],
            self.mem[a + 2],
            self.mem[a + 3],
        ]))
    }

    fn write_word(&mut self, addr: u32, value: i32) -> bool {
        let a = addr as usize;
        if a + 4 > self.mem.len() {
            return false;
        }
        self.mem[a..a + 4].copy_from_slice(&value.to_le_bytes());
        true
    }

    fn write_reg(&mut self, delta: &mut StepDelta, reg: u8, value: i32) {
        if reg == 0 {
            return; // x0 is hardwired to zero
        }
        let old = self.regs[reg as usize];
        self.regs[reg as usize] = value;
        delta.reg_write = Some(RegWrite { reg, old, new: value });
    }

    /// One fetch-decode-execute cycle.
    pub fn step(&mut self) -> StepDelta {
        let pc = self.pc;
        let mut delta = StepDelta {
            pc_before: pc,
            pc_after: pc,
            instr_index: pc / 4,
            reg_write: None,
            mem_write: None,
            output: None,
            halted: self.halted,
            error: None,
        };

        if self.halted {
            return delta;
        }

        // FETCH - read 4 bytes from memory at the program counter.
        let word = match self.read_word(pc) {
            Some(w) => w as u32,
            None => return self.fault(delta, format!("fetch out of bounds at {pc:#06x}")),
        };
        if pc % 4 != 0 {
            return self.fault(delta, format!("misaligned PC {pc:#06x}"));
        }

        // DECODE - parse the bits back into an instruction.
        let instr = match decode(word) {
            Some(i) => i,
            None => {
                return self.fault(
                    delta,
                    format!("cannot decode word {word:#010x} at {pc:#06x}"),
                )
            }
        };

        // EXECUTE - apply the effect.
        let mut next_pc = pc.wrapping_add(4);
        let r = |i: &Cpu, reg: u8| i.regs[reg as usize];

        use Instr::*;
        match instr {
            Lui { rd, imm } => {
                self.write_reg(&mut delta, rd, imm << 12);
            }
            Addi { rd, rs1, imm } => {
                self.write_reg(&mut delta, rd, r(self, rs1).wrapping_add(imm));
            }
            Sltiu { rd, rs1, imm } => {
                let v = ((r(self, rs1) as u32) < (imm as u32)) as i32;
                self.write_reg(&mut delta, rd, v);
            }
            Xori { rd, rs1, imm } => {
                self.write_reg(&mut delta, rd, r(self, rs1) ^ imm);
            }
            Add { rd, rs1, rs2 } => {
                self.write_reg(&mut delta, rd, r(self, rs1).wrapping_add(r(self, rs2)));
            }
            Sub { rd, rs1, rs2 } => {
                self.write_reg(&mut delta, rd, r(self, rs1).wrapping_sub(r(self, rs2)));
            }
            Mul { rd, rs1, rs2 } => {
                self.write_reg(&mut delta, rd, r(self, rs1).wrapping_mul(r(self, rs2)));
            }
            Div { rd, rs1, rs2 } => {
                // RISC-V semantics: x/0 = -1, overflow wraps
                let (a, b) = (r(self, rs1), r(self, rs2));
                let v = if b == 0 { -1 } else { a.wrapping_div(b) };
                self.write_reg(&mut delta, rd, v);
            }
            Rem { rd, rs1, rs2 } => {
                // RISC-V semantics: x%0 = x
                let (a, b) = (r(self, rs1), r(self, rs2));
                let v = if b == 0 { a } else { a.wrapping_rem(b) };
                self.write_reg(&mut delta, rd, v);
            }
            Slt { rd, rs1, rs2 } => {
                self.write_reg(&mut delta, rd, (r(self, rs1) < r(self, rs2)) as i32);
            }
            Sltu { rd, rs1, rs2 } => {
                let v = ((r(self, rs1) as u32) < (r(self, rs2) as u32)) as i32;
                self.write_reg(&mut delta, rd, v);
            }
            Lw { rd, rs1, imm } => {
                let addr = (r(self, rs1).wrapping_add(imm)) as u32;
                match self.read_word(addr) {
                    Some(v) => self.write_reg(&mut delta, rd, v),
                    None => return self.fault(delta, format!("load out of bounds at {addr:#06x}")),
                }
            }
            Sw { rs1, rs2, imm } => {
                let addr = (r(self, rs1).wrapping_add(imm)) as u32;
                let old = match self.read_word(addr) {
                    Some(v) => v,
                    None => {
                        return self.fault(delta, format!("store out of bounds at {addr:#06x}"))
                    }
                };
                let new = r(self, rs2);
                self.write_word(addr, new);
                delta.mem_write = Some(MemWrite { addr, old, new });
            }
            Beq { rs1, rs2, offset } => {
                if r(self, rs1) == r(self, rs2) {
                    next_pc = pc.wrapping_add(offset as u32);
                }
            }
            Bne { rs1, rs2, offset } => {
                if r(self, rs1) != r(self, rs2) {
                    next_pc = pc.wrapping_add(offset as u32);
                }
            }
            Blt { rs1, rs2, offset } => {
                if r(self, rs1) < r(self, rs2) {
                    next_pc = pc.wrapping_add(offset as u32);
                }
            }
            Bge { rs1, rs2, offset } => {
                if r(self, rs1) >= r(self, rs2) {
                    next_pc = pc.wrapping_add(offset as u32);
                }
            }
            Jal { rd, offset } => {
                self.write_reg(&mut delta, rd, pc.wrapping_add(4) as i32);
                next_pc = pc.wrapping_add(offset as u32);
            }
            Ecall => {
                let call = self.regs[17]; // a7
                match call {
                    SYSCALL_PRINT_INT => {
                        delta.output = Some(format!("{}\n", self.regs[10])); // a0
                    }
                    SYSCALL_EXIT => {
                        self.halted = true;
                        delta.halted = true;
                    }
                    other => {
                        return self.fault(delta, format!("unknown ecall number {other}"));
                    }
                }
            }
        }

        self.pc = next_pc;
        delta.pc_after = next_pc;
        delta
    }

    fn fault(&mut self, mut delta: StepDelta, message: String) -> StepDelta {
        self.halted = true;
        delta.halted = true;
        delta.error = Some(message);
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assembler::encode;
    use crate::ir::Instr::*;

    fn program(instrs: &[Instr]) -> Vec<u8> {
        instrs.iter().flat_map(|i| encode(*i).to_le_bytes()).collect()
    }

    #[test]
    fn executes_arithmetic() {
        let mut cpu = Cpu::new(&program(&[
            Addi { rd: 5, rs1: 0, imm: 6 },
            Addi { rd: 6, rs1: 0, imm: 7 },
            Mul { rd: 7, rs1: 5, rs2: 6 },
        ]));
        cpu.step();
        cpu.step();
        let d = cpu.step();
        assert_eq!(cpu.regs[7], 42);
        let w = d.reg_write.unwrap();
        assert_eq!((w.reg, w.old, w.new), (7, 0, 42));
        assert_eq!(d.pc_after, 12);
    }

    #[test]
    fn x0_stays_zero() {
        let mut cpu = Cpu::new(&program(&[Addi { rd: 0, rs1: 0, imm: 99 }]));
        let d = cpu.step();
        assert_eq!(cpu.regs[0], 0);
        assert!(d.reg_write.is_none());
    }

    #[test]
    fn branch_taken_and_not_taken() {
        // beq x0, x0 → always taken, jumps over the addi
        let mut cpu = Cpu::new(&program(&[
            Beq { rs1: 0, rs2: 0, offset: 8 },
            Addi { rd: 5, rs1: 0, imm: 1 }, // skipped
            Addi { rd: 6, rs1: 0, imm: 2 },
        ]));
        let d = cpu.step();
        assert_eq!(d.pc_after, 8);
        cpu.step();
        assert_eq!(cpu.regs[5], 0);
        assert_eq!(cpu.regs[6], 2);
    }

    #[test]
    fn store_load_roundtrip_with_delta() {
        let mut cpu = Cpu::new(&program(&[
            Addi { rd: 5, rs1: 0, imm: 77 },
            Sw { rs1: 0, rs2: 5, imm: 100 },
            Lw { rd: 6, rs1: 0, imm: 100 },
        ]));
        cpu.step();
        let d = cpu.step();
        let w = d.mem_write.unwrap();
        assert_eq!((w.addr, w.old, w.new), (100, 0, 77));
        cpu.step();
        assert_eq!(cpu.regs[6], 77);
    }

    #[test]
    fn print_and_exit_ecalls() {
        let mut cpu = Cpu::new(&program(&[
            Addi { rd: 10, rs1: 0, imm: 42 }, // a0 = 42
            Addi { rd: 17, rs1: 0, imm: 1 },  // a7 = print
            Ecall,
            Addi { rd: 17, rs1: 0, imm: 10 }, // a7 = exit
            Ecall,
        ]));
        cpu.step();
        cpu.step();
        let d = cpu.step();
        assert_eq!(d.output.as_deref(), Some("42\n"));
        cpu.step();
        let d = cpu.step();
        assert!(d.halted);
        assert!(cpu.halted);
        // further steps are inert
        let d = cpu.step();
        assert!(d.halted);
        assert!(d.reg_write.is_none());
    }

    #[test]
    fn division_follows_riscv_semantics() {
        let mut cpu = Cpu::new(&program(&[
            Addi { rd: 5, rs1: 0, imm: 7 },
            Div { rd: 6, rs1: 5, rs2: 0 }, // 7 / 0 = -1
            Rem { rd: 7, rs1: 5, rs2: 0 }, // 7 % 0 = 7
        ]));
        cpu.step();
        cpu.step();
        cpu.step();
        assert_eq!(cpu.regs[6], -1);
        assert_eq!(cpu.regs[7], 7);
    }

    #[test]
    fn undecodable_word_faults() {
        let mut cpu = Cpu::new(&[0, 0, 0, 0]);
        let d = cpu.step();
        assert!(d.error.is_some());
        assert!(d.halted);
    }
}
