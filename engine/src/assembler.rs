//! Assembler: `Instr` → genuine 32-bit RV32IM machine words, and the decoder
//! that turns words back into instructions.
//!
//! The decoder is what the CPU simulator runs on - it decodes the *actual
//! bytes in memory* every cycle, so the machine-code panel isn't decoration:
//! it is literally what executes.

use crate::ir::*;

// opcodes
const OP_LUI: u32 = 0b0110111;
const OP_IMM: u32 = 0b0010011; // addi, sltiu, xori
const OP_REG: u32 = 0b0110011; // add, sub, mul, ...
const OP_LOAD: u32 = 0b0000011; // lw
const OP_STORE: u32 = 0b0100011; // sw
const OP_BRANCH: u32 = 0b1100011; // beq, bne, blt, bge
const OP_JAL: u32 = 0b1101111;
const OP_SYSTEM: u32 = 0b1110011; // ecall

pub fn encode(instr: Instr) -> u32 {
    use Instr::*;
    match instr {
        Lui { rd, imm } => ((imm as u32 & 0xfffff) << 12) | rd_bits(rd) | OP_LUI,

        Addi { rd, rs1, imm } => i_type(imm, rs1, 0b000, rd, OP_IMM),
        Sltiu { rd, rs1, imm } => i_type(imm, rs1, 0b011, rd, OP_IMM),
        Xori { rd, rs1, imm } => i_type(imm, rs1, 0b100, rd, OP_IMM),

        Add { rd, rs1, rs2 } => r_type(0b0000000, rs2, rs1, 0b000, rd),
        Sub { rd, rs1, rs2 } => r_type(0b0100000, rs2, rs1, 0b000, rd),
        Slt { rd, rs1, rs2 } => r_type(0b0000000, rs2, rs1, 0b010, rd),
        Sltu { rd, rs1, rs2 } => r_type(0b0000000, rs2, rs1, 0b011, rd),
        Mul { rd, rs1, rs2 } => r_type(0b0000001, rs2, rs1, 0b000, rd),
        Div { rd, rs1, rs2 } => r_type(0b0000001, rs2, rs1, 0b100, rd),
        Rem { rd, rs1, rs2 } => r_type(0b0000001, rs2, rs1, 0b110, rd),

        Lw { rd, rs1, imm } => i_type(imm, rs1, 0b010, rd, OP_LOAD),
        Sw { rs1, rs2, imm } => s_type(imm, rs2, rs1, 0b010),

        Beq { rs1, rs2, offset } => b_type(offset, rs2, rs1, 0b000),
        Bne { rs1, rs2, offset } => b_type(offset, rs2, rs1, 0b001),
        Blt { rs1, rs2, offset } => b_type(offset, rs2, rs1, 0b100),
        Bge { rs1, rs2, offset } => b_type(offset, rs2, rs1, 0b101),

        Jal { rd, offset } => j_type(offset, rd),

        Ecall => OP_SYSTEM,
    }
}

pub fn decode(word: u32) -> Option<Instr> {
    let opcode = word & 0x7f;
    let rd = ((word >> 7) & 0x1f) as Reg;
    let funct3 = (word >> 12) & 0x7;
    let rs1 = ((word >> 15) & 0x1f) as Reg;
    let rs2 = ((word >> 20) & 0x1f) as Reg;
    let funct7 = word >> 25;

    use Instr::*;
    match opcode {
        OP_LUI => Some(Lui { rd, imm: (word >> 12) as i32 }),
        OP_IMM => {
            let imm = i_imm(word);
            match funct3 {
                0b000 => Some(Addi { rd, rs1, imm }),
                0b011 => Some(Sltiu { rd, rs1, imm }),
                0b100 => Some(Xori { rd, rs1, imm }),
                _ => None,
            }
        }
        OP_REG => match (funct7, funct3) {
            (0b0000000, 0b000) => Some(Add { rd, rs1, rs2 }),
            (0b0100000, 0b000) => Some(Sub { rd, rs1, rs2 }),
            (0b0000000, 0b010) => Some(Slt { rd, rs1, rs2 }),
            (0b0000000, 0b011) => Some(Sltu { rd, rs1, rs2 }),
            (0b0000001, 0b000) => Some(Mul { rd, rs1, rs2 }),
            (0b0000001, 0b100) => Some(Div { rd, rs1, rs2 }),
            (0b0000001, 0b110) => Some(Rem { rd, rs1, rs2 }),
            _ => None,
        },
        OP_LOAD if funct3 == 0b010 => Some(Lw { rd, rs1, imm: i_imm(word) }),
        OP_STORE if funct3 == 0b010 => Some(Sw { rs1, rs2, imm: s_imm(word) }),
        OP_BRANCH => {
            let offset = b_imm(word);
            match funct3 {
                0b000 => Some(Beq { rs1, rs2, offset }),
                0b001 => Some(Bne { rs1, rs2, offset }),
                0b100 => Some(Blt { rs1, rs2, offset }),
                0b101 => Some(Bge { rs1, rs2, offset }),
                _ => None,
            }
        }
        OP_JAL => Some(Jal { rd, offset: j_imm(word) }),
        OP_SYSTEM if word == OP_SYSTEM => Some(Ecall),
        _ => None,
    }
}

// ---- field packing -------------------------------------------------------

fn rd_bits(rd: Reg) -> u32 {
    (rd as u32) << 7
}

fn r_type(funct7: u32, rs2: Reg, rs1: Reg, funct3: u32, rd: Reg) -> u32 {
    (funct7 << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | (funct3 << 12)
        | rd_bits(rd)
        | OP_REG
}

fn i_type(imm: i32, rs1: Reg, funct3: u32, rd: Reg, opcode: u32) -> u32 {
    debug_assert!((-2048..=2047).contains(&imm), "i-type imm out of range: {imm}");
    ((imm as u32 & 0xfff) << 20) | ((rs1 as u32) << 15) | (funct3 << 12) | rd_bits(rd) | opcode
}

fn s_type(imm: i32, rs2: Reg, rs1: Reg, funct3: u32) -> u32 {
    debug_assert!((-2048..=2047).contains(&imm), "s-type imm out of range: {imm}");
    let imm = imm as u32 & 0xfff;
    ((imm >> 5) << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | (funct3 << 12)
        | ((imm & 0x1f) << 7)
        | OP_STORE
}

fn b_type(offset: i32, rs2: Reg, rs1: Reg, funct3: u32) -> u32 {
    debug_assert!(offset % 2 == 0, "branch offset must be even");
    debug_assert!((-4096..=4094).contains(&offset), "b-type offset out of range: {offset}");
    let imm = offset as u32 & 0x1fff;
    (((imm >> 12) & 1) << 31)
        | (((imm >> 5) & 0x3f) << 25)
        | ((rs2 as u32) << 20)
        | ((rs1 as u32) << 15)
        | (funct3 << 12)
        | (((imm >> 1) & 0xf) << 8)
        | (((imm >> 11) & 1) << 7)
        | OP_BRANCH
}

fn j_type(offset: i32, rd: Reg) -> u32 {
    debug_assert!(offset % 2 == 0, "jump offset must be even");
    debug_assert!((-(1 << 20)..(1 << 20)).contains(&offset), "j-type offset out of range");
    let imm = offset as u32 & 0x1fffff;
    (((imm >> 20) & 1) << 31)
        | (((imm >> 1) & 0x3ff) << 21)
        | (((imm >> 11) & 1) << 20)
        | (((imm >> 12) & 0xff) << 12)
        | rd_bits(rd)
        | OP_JAL
}

// ---- immediate extraction (sign-extended) ---------------------------------

fn i_imm(word: u32) -> i32 {
    (word as i32) >> 20
}

fn s_imm(word: u32) -> i32 {
    (((word as i32) >> 25) << 5) | (((word >> 7) & 0x1f) as i32)
}

fn b_imm(word: u32) -> i32 {
    let sign = (word as i32) >> 31; // imm[12]
    let b11 = ((word >> 7) & 1) as i32;
    let b10_5 = ((word >> 25) & 0x3f) as i32;
    let b4_1 = ((word >> 8) & 0xf) as i32;
    (sign << 12) | (b11 << 11) | (b10_5 << 5) | (b4_1 << 1)
}

fn j_imm(word: u32) -> i32 {
    let sign = (word as i32) >> 31; // imm[20]
    let b19_12 = ((word >> 12) & 0xff) as i32;
    let b11 = ((word >> 20) & 1) as i32;
    let b10_1 = ((word >> 21) & 0x3ff) as i32;
    (sign << 20) | (b19_12 << 12) | (b11 << 11) | (b10_1 << 1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::Instr::*;

    /// Encodings cross-checked against the RISC-V spec / riscv-tests.
    #[test]
    fn known_good_words() {
        assert_eq!(encode(Addi { rd: 5, rs1: 0, imm: 7 }), 0x00700293);
        assert_eq!(encode(Add { rd: 5, rs1: 6, rs2: 7 }), 0x007302B3);
        assert_eq!(encode(Lui { rd: 3, imm: 1 }), 0x000011B7);
        assert_eq!(encode(Lw { rd: 5, rs1: 3, imm: 8 }), 0x0081A283);
        assert_eq!(encode(Sw { rs1: 3, rs2: 5, imm: 8 }), 0x0051A423);
        assert_eq!(encode(Beq { rs1: 5, rs2: 0, offset: 8 }), 0x00028463);
        assert_eq!(encode(Jal { rd: 0, offset: 8 }), 0x0080006F);
        assert_eq!(encode(Ecall), 0x00000073);
    }

    #[test]
    fn encode_decode_roundtrip() {
        let cases = vec![
            Lui { rd: 3, imm: 0x12345 },
            Lui { rd: 3, imm: 0xfffff }, // negative %hi after patching
            Addi { rd: 5, rs1: 0, imm: -2048 },
            Addi { rd: 5, rs1: 0, imm: 2047 },
            Sltiu { rd: 5, rs1: 6, imm: 1 },
            Xori { rd: 5, rs1: 5, imm: 1 },
            Add { rd: 5, rs1: 6, rs2: 7 },
            Sub { rd: 31, rs1: 30, rs2: 29 },
            Mul { rd: 5, rs1: 6, rs2: 7 },
            Div { rd: 5, rs1: 6, rs2: 7 },
            Rem { rd: 5, rs1: 6, rs2: 7 },
            Slt { rd: 5, rs1: 6, rs2: 7 },
            Sltu { rd: 5, rs1: 0, rs2: 7 },
            Lw { rd: 5, rs1: 3, imm: -4 },
            Sw { rs1: 3, rs2: 5, imm: -4 },
            Beq { rs1: 5, rs2: 0, offset: -8 },
            Bne { rs1: 5, rs2: 6, offset: 4094 },
            Blt { rs1: 5, rs2: 6, offset: -4096 },
            Bge { rs1: 5, rs2: 6, offset: 12 },
            Jal { rd: 0, offset: -2048 },
            Jal { rd: 1, offset: 1048574 },
            Ecall,
        ];
        for instr in cases {
            let word = encode(instr);
            let back = decode(word);
            assert_eq!(back, Some(instr), "roundtrip failed for {instr:?} (word {word:#010x})");
        }
    }

    #[test]
    fn garbage_decodes_to_none() {
        assert_eq!(decode(0x00000000), None);
        assert_eq!(decode(0xffffffff), None);
    }
}
