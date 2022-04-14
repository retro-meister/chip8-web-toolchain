use crate::compiler::Opcode::*;
use crate::compiler::*;
use crate::lexer::TokenType::*;
use crate::lexer::*;
use crate::utils;

use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub struct Assembler {
    asm: Vec<Opcode>,
    binary_u16: Vec<u16>,
    binary: Vec<u8>,
}

#[wasm_bindgen]
impl Assembler {
    pub fn new_from_compiler(compiler: &Compiler) -> Assembler {
        Assembler {
            asm: compiler.asm().clone(),
            binary_u16: Vec::new(),
            binary: Vec::new(),
        }
    }

    fn opcode_to_u16(op: &Opcode) -> u16 {
        match op {
            LDRegByte(reg, byte) => (0x6 << 12) | (reg << 8) | (byte),
            LDRegReg(reg1, reg2) => (0x8 << 12) | (reg1 << 8) | (reg2 << 4) | (0x0),
            AddRegReg(reg1, reg2) => (0x8 << 12) | (reg1 << 8) | (reg2 << 4) | (0x4),
            SubRegReg(reg1, reg2) => (0x8 << 12) | (reg1 << 8) | (reg2 << 4) | (0x5),
            SERegReg(reg1, reg2) => (0x5 << 12) | (reg1 << 8) | (reg2 << 4) | (0x0),
            SNERegReg(reg1, reg2) => (0x9 << 12) | (reg1 << 8) | (reg2 << 4) | (0x0),
            LDFReg(reg) => (0xF << 12) | (reg << 8) | (0x29),
            LDIReg(reg) => (0xF << 12) | (reg << 8) | (0x55),
            LDRegI(reg) => (0xF << 12) | (reg << 8) | (0x65),
            LDDTReg(reg) => (0xF << 12) | (reg << 8) | (0x15),
            LDRegDT(reg) => (0xF << 12) | (reg << 8) | (0x07),
            LDSTReg(reg) => (0xF << 12) | (reg << 8) | (0x18),
            LDRegKey(reg) => (0xF << 12) | (reg << 8) | (0x0A),
            LDIAddr(addr) => (0xA << 12) | (addr),
            RNDRegByte(reg, byte) => (0xC << 12) | (reg << 8) | (byte),
            DRWRegRegNibble(reg1, reg2, nib) => (0xD << 12) | (reg1 << 8) | (reg2 << 4) | (nib),
            JP(addr) => (0x1 << 12) | (addr),
            CALL(addr) => (0x2 << 12) | (addr),
            RET => 0x00EE,
        }
    }

    pub fn assemble(&mut self) {
        for cur in self.asm.iter() {
            let bytes = Assembler::opcode_to_u16(cur);
            self.binary_u16.push(bytes);
            let split = bytes.to_be_bytes();
            self.binary.push(split[0]);
            self.binary.push(split[1]);
        }
    }

    pub fn stringify_binary(&self) -> String {
        self.binary_u16
            .iter()
            //.map(|byte| byte.to_string())
            .map(|byte| format!("{:0>2X}", byte))
            .collect::<Vec<String>>()
            .join(" ")
    }
}

impl Assembler {
    pub fn binary(&self) -> &Vec<u8> {
        &self.binary
    }
}

#[cfg(test)]
mod tests {
    use super::Assembler;
    use super::*;

    #[test]
    pub fn test_opcode_to_u16() {
        //println!("{}", Assembler::opcode_to_u16(&LDRegByte(0, 0xD)));
        assert_eq!(Assembler::opcode_to_u16(&LDRegByte(0, 0xD)), 0x600D);
        assert_eq!(Assembler::opcode_to_u16(&AddRegReg(4, 15)), 0x84F4);
    }

    #[test]
    pub fn test_assemble() {
        let mut l = Lexer::new("14 + 14;");
        l.lex();

        let mut c = Compiler::new_from_lexer(&l);

        c.compile();
        //println!("{}", c.stringify_asm());

        let mut a = Assembler::new_from_compiler(&c);
        a.assemble();

        assert!(utils::vectors_equivalent(
            a.binary,
            vec![0x60, 0x0E, 0x61, 0x0E, 0x80, 0x14,]
        ));
    }

    #[test]
    pub fn test_sub() {
        let mut l = Lexer::new("9 - 7;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        let mut a = Assembler::new_from_compiler(&c);
        a.assemble();

        assert!(utils::vectors_equivalent(
            a.binary,
            vec![0x60, 0x09, 0x61, 0x07, 0x80, 0x15]
        ));
    }
}
