//! The RV32IM instruction subset the compiler targets.
//!
//! Real RISC-V: these instructions assemble to genuine 32-bit RV32IM words
//! (see `assembler.rs`) and any RISC-V toolchain would agree on the encoding.

use serde::Serialize;

/// A register index x0..x31. x0 is hardwired to zero.
pub type Reg = u8;

pub const ZERO: Reg = 0;
pub const GP: Reg = 3; // global pointer — base address of variable slots
pub const A0: Reg = 10; // syscall argument
pub const A7: Reg = 17; // syscall number
/// Temporaries used as the expression evaluation stack: t0-t6.
pub const TEMPS: [Reg; 7] = [5, 6, 7, 28, 29, 30, 31];

/// RISC-V ABI register names, indexed by register number.
pub const REG_NAMES: [&str; 32] = [
    "zero", "ra", "sp", "gp", "tp", "t0", "t1", "t2", "s0", "s1", "a0", "a1", "a2", "a3", "a4",
    "a5", "a6", "a7", "s2", "s3", "s4", "s5", "s6", "s7", "s8", "s9", "s10", "s11", "t3", "t4",
    "t5", "t6",
];

pub fn reg_name(r: Reg) -> &'static str {
    REG_NAMES[r as usize]
}

/// Environment-call numbers (placed in a7 before `ecall`).
pub const SYSCALL_PRINT_INT: i32 = 1;
pub const SYSCALL_EXIT: i32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum Instr {
    // U-type. `imm` is the 20-bit value placed in bits [31:12] of the register.
    Lui { rd: Reg, imm: i32 },
    // I-type arithmetic
    Addi { rd: Reg, rs1: Reg, imm: i32 },
    Sltiu { rd: Reg, rs1: Reg, imm: i32 },
    Xori { rd: Reg, rs1: Reg, imm: i32 },
    // R-type
    Add { rd: Reg, rs1: Reg, rs2: Reg },
    Sub { rd: Reg, rs1: Reg, rs2: Reg },
    Mul { rd: Reg, rs1: Reg, rs2: Reg },
    Div { rd: Reg, rs1: Reg, rs2: Reg },
    Rem { rd: Reg, rs1: Reg, rs2: Reg },
    Slt { rd: Reg, rs1: Reg, rs2: Reg },
    Sltu { rd: Reg, rs1: Reg, rs2: Reg },
    // loads/stores
    Lw { rd: Reg, rs1: Reg, imm: i32 },
    Sw { rs1: Reg, rs2: Reg, imm: i32 }, // sw rs2, imm(rs1)
    // branches — `offset` is a byte offset relative to this instruction
    Beq { rs1: Reg, rs2: Reg, offset: i32 },
    Bne { rs1: Reg, rs2: Reg, offset: i32 },
    Blt { rs1: Reg, rs2: Reg, offset: i32 },
    Bge { rs1: Reg, rs2: Reg, offset: i32 },
    // jump — `offset` is a byte offset relative to this instruction
    Jal { rd: Reg, offset: i32 },
    // environment call
    Ecall,
}

impl Instr {
    /// Human-readable assembly text. `addr` is the instruction's own address,
    /// used to render branch/jump targets as absolute addresses.
    pub fn text(&self, addr: u32) -> String {
        use Instr::*;
        let target = |offset: i32| (addr as i64 + offset as i64) as u32;
        match *self {
            Lui { rd, imm } => format!("lui {}, {:#x}", reg_name(rd), imm),
            Addi { rd, rs1, imm } => {
                format!("addi {}, {}, {}", reg_name(rd), reg_name(rs1), imm)
            }
            Sltiu { rd, rs1, imm } => {
                format!("sltiu {}, {}, {}", reg_name(rd), reg_name(rs1), imm)
            }
            Xori { rd, rs1, imm } => {
                format!("xori {}, {}, {}", reg_name(rd), reg_name(rs1), imm)
            }
            Add { rd, rs1, rs2 } => r_text("add", rd, rs1, rs2),
            Sub { rd, rs1, rs2 } => r_text("sub", rd, rs1, rs2),
            Mul { rd, rs1, rs2 } => r_text("mul", rd, rs1, rs2),
            Div { rd, rs1, rs2 } => r_text("div", rd, rs1, rs2),
            Rem { rd, rs1, rs2 } => r_text("rem", rd, rs1, rs2),
            Slt { rd, rs1, rs2 } => r_text("slt", rd, rs1, rs2),
            Sltu { rd, rs1, rs2 } => r_text("sltu", rd, rs1, rs2),
            Lw { rd, rs1, imm } => {
                format!("lw {}, {}({})", reg_name(rd), imm, reg_name(rs1))
            }
            Sw { rs1, rs2, imm } => {
                format!("sw {}, {}({})", reg_name(rs2), imm, reg_name(rs1))
            }
            Beq { rs1, rs2, offset } => b_text("beq", rs1, rs2, target(offset)),
            Bne { rs1, rs2, offset } => b_text("bne", rs1, rs2, target(offset)),
            Blt { rs1, rs2, offset } => b_text("blt", rs1, rs2, target(offset)),
            Bge { rs1, rs2, offset } => b_text("bge", rs1, rs2, target(offset)),
            Jal { rd, offset } => {
                if rd == ZERO {
                    format!("j {:#06x}", target(offset))
                } else {
                    format!("jal {}, {:#06x}", reg_name(rd), target(offset))
                }
            }
            Ecall => "ecall".to_string(),
        }
    }
}

fn r_text(op: &str, rd: Reg, rs1: Reg, rs2: Reg) -> String {
    format!("{} {}, {}, {}", op, reg_name(rd), reg_name(rs1), reg_name(rs2))
}

fn b_text(op: &str, rs1: Reg, rs2: Reg, target: u32) -> String {
    format!("{} {}, {}, {:#06x}", op, reg_name(rs1), reg_name(rs2), target)
}
