use crate::assembler::*;
use crate::utils;

use array_init::array_init;
use getrandom::*;
use std::collections::HashMap;
use std::fs::File;
use std::io;
use std::io::Read;
use wasm_bindgen::prelude::*;

type Chip8OpcodeFn = fn(&mut Chip8);
type GetNameFn = fn(&mut Chip8) -> String;

pub struct Instruction {
    get_disasm: GetNameFn,
    operation: Chip8OpcodeFn,
}

#[derive(Debug, Clone)]
pub struct Chip8State {
    //next opcode for fetch-execute-decode cycle
    opcode: u16,
    //16 general purpose registers
    V: [u8; 16],
    //index register
    I: u16,
    //program counter
    pc: u16,
    //64*32 framebuffer
    framebuffer: [u32; 64 * 32],
    //timers
    delay_timer: u8,
    sound_timer: u8,
    //stack
    stack: [u16; 16],
    //stack pointer
    sp: u8,
    //key status
    keys: [u8; 16],
    //4096 bytes of addressable memory
    ram: [u8; 4096],
}

impl Chip8State {
    pub fn new() -> Chip8State {
        Chip8State {
            opcode: 0,
            V: [0; 16],
            I: 0,
            pc: 0,
            framebuffer: [0; 64 * 32],
            delay_timer: 0,
            sound_timer: 0,
            stack: [0; 16],
            sp: 0,
            keys: [0; 16],
            ram: [0; 4096],
        }
    }
}

#[wasm_bindgen]
pub struct Chip8 {
    state: Chip8State,

    saved_state: Chip8State,

    //chip built-in fontset
    fontset: [u8; 80],

    video_width: u32,
    video_height: u32,

    disasm_opcode: u16,

    opcodes: [Instruction; 0xF + 1],
    opcodes_0: [Instruction; 0xE + 1],
    opcodes_8: [Instruction; 0xE + 1],
    opcodes_E: [Instruction; 0xE + 1],
    opcodes_F: [Instruction; 0x65 + 1],

    disasm_map: HashMap<u16, String>,
}

#[wasm_bindgen]
impl Chip8 {
    pub fn new() -> Chip8 {
        utils::set_panic_hook();

        let opcodes = [
            Instruction {
                get_disasm: Chip8::opcodes_0_name_lookup,
                operation: Chip8::opcodes_0_lookup,
            },
            Instruction {
                get_disasm: |c8| format!("JP {}", Chip8::get_args_disasm_nnn(c8)),
                operation: Chip8::OP_1nnn,
            },
            Instruction {
                get_disasm: |c8| format!("CALL {}", Chip8::get_args_disasm_nnn(c8)),
                operation: Chip8::OP_2nnn,
            },
            Instruction {
                get_disasm: |c8| format!("SE {}", Chip8::get_args_disasm_xkk(c8)),
                operation: Chip8::OP_3xkk,
            },
            Instruction {
                get_disasm: |c8| format!("SNE {}", Chip8::get_args_disasm_xkk(c8)),
                operation: Chip8::OP_4xkk,
            },
            Instruction {
                get_disasm: |c8| format!("SE {}", Chip8::get_args_disasm_xy(c8)),
                operation: Chip8::OP_5xy0,
            },
            Instruction {
                get_disasm: |c8| format!("LD {}", Chip8::get_args_disasm_xkk(c8)),
                operation: Chip8::OP_6xkk,
            },
            Instruction {
                get_disasm: |c8| format!("ADD {}", Chip8::get_args_disasm_xkk(c8)),
                operation: Chip8::OP_7xkk,
            },
            Instruction {
                get_disasm: Chip8::opcodes_8_name_lookup,
                operation: Chip8::opcodes_8_lookup,
            },
            Instruction {
                get_disasm: |c8| format!("SNE {}", Chip8::get_args_disasm_xy(c8)),
                operation: Chip8::OP_9xy0,
            },
            Instruction {
                get_disasm: |c8| format!("LD I, {}", Chip8::get_args_disasm_nnn(c8)),
                operation: Chip8::OP_Annn,
            },
            Instruction {
                get_disasm: |c8| format!("JP V0, {}", Chip8::get_args_disasm_nnn(c8)),
                operation: Chip8::OP_Bnnn,
            },
            Instruction {
                get_disasm: |c8| format!("RND {}", Chip8::get_args_disasm_xkk(c8)),
                operation: Chip8::OP_Cxkk,
            },
            Instruction {
                get_disasm: |c8| format!("DRW {}", Chip8::get_args_disasm_xyn(c8)),
                operation: Chip8::OP_Dxyn,
            },
            Instruction {
                get_disasm: Chip8::opcodes_E_name_lookup,
                operation: Chip8::opcodes_E_lookup,
            },
            Instruction {
                get_disasm: Chip8::opcodes_F_name_lookup,
                operation: Chip8::opcodes_F_lookup,
            },
        ];

        let mut opcodes_0: [Instruction; 0xE + 1] = array_init(|_i| Instruction {
            get_disasm: |_| String::from("null"),
            operation: Chip8::OP_null,
        });
        opcodes_0[0x0] = Instruction {
            get_disasm: |_| String::from("CLS"),
            operation: Chip8::OP_00E0,
        };
        opcodes_0[0xE] = Instruction {
            get_disasm: |_| String::from("RET"),
            operation: Chip8::OP_00EE,
        };

        let mut opcodes_8: [Instruction; 0xE + 1] = array_init(|_i| Instruction {
            get_disasm: |_| String::from("null"),
            operation: Chip8::OP_null,
        });
        opcodes_8[0x0] = Instruction {
            get_disasm: |c8| format!("LD {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy0,
        };
        opcodes_8[0x1] = Instruction {
            get_disasm: |c8| format!("OR {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy1,
        };
        opcodes_8[0x2] = Instruction {
            get_disasm: |c8| format!("AND {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy2,
        };
        opcodes_8[0x3] = Instruction {
            get_disasm: |c8| format!("XOR {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy3,
        };
        opcodes_8[0x4] = Instruction {
            get_disasm: |c8| format!("ADD {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy4,
        };
        opcodes_8[0x5] = Instruction {
            get_disasm: |c8| format!("SUB {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy5,
        };
        opcodes_8[0x6] = Instruction {
            get_disasm: |c8| format!("SHR {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy6,
        };
        opcodes_8[0x7] = Instruction {
            get_disasm: |c8| format!("SUBN {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xy7,
        };
        opcodes_8[0xE] = Instruction {
            get_disasm: |c8| format!("SHL {}", Chip8::get_args_disasm_xy(c8)),
            operation: Chip8::OP_8xyE,
        };

        let mut opcodes_E: [Instruction; 0xE + 1] = array_init(|_i| Instruction {
            get_disasm: |_| String::from("null"),
            operation: Chip8::OP_null,
        });
        opcodes_E[0xE] = Instruction {
            get_disasm: |c8| format!("SKP {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Ex9E,
        };
        opcodes_E[0x1] = Instruction {
            get_disasm: |c8| format!("SKNP {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_ExA1,
        };

        let mut opcodes_F: [Instruction; 0x65 + 1] = array_init(|_i| Instruction {
            get_disasm: |_| String::from("null"),
            operation: Chip8::OP_null,
        });
        opcodes_F[0x07] = Instruction {
            get_disasm: |c8| format!("LD {}, DT", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx07,
        };
        opcodes_F[0x0A] = Instruction {
            get_disasm: |c8| format!("LD {}, K", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx0A,
        };
        opcodes_F[0x15] = Instruction {
            get_disasm: |c8| format!("LD DT, {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx15,
        };
        opcodes_F[0x18] = Instruction {
            get_disasm: |c8| format!("LD ST, {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx18,
        };
        opcodes_F[0x1E] = Instruction {
            get_disasm: |c8| format!("ADD I, {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx1E,
        };
        opcodes_F[0x29] = Instruction {
            get_disasm: |c8| format!("LD F, {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx29,
        };
        opcodes_F[0x33] = Instruction {
            get_disasm: |c8| format!("LD B, {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx33,
        };
        opcodes_F[0x55] = Instruction {
            get_disasm: |c8| format!("LD [I], {}", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx55,
        };
        opcodes_F[0x65] = Instruction {
            get_disasm: |c8| format!("LD {}, [I]", Chip8::get_args_disasm_x(c8)),
            operation: Chip8::OP_Fx65,
        };

        Chip8 {
            state: Chip8State::new(),
            saved_state: Chip8State::new(),

            //fontset: [0; 80],
            fontset: [
                0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
                0x20, 0x60, 0x20, 0x20, 0x70, // 1
                0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
                0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
                0x90, 0x90, 0xF0, 0x10, 0x10, // 4
                0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
                0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
                0xF0, 0x10, 0x20, 0x40, 0x40, // 7
                0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
                0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
                0xF0, 0x90, 0xF0, 0x90, 0x90, // A
                0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
                0xF0, 0x80, 0x80, 0x80, 0xF0, // C
                0xE0, 0x90, 0x90, 0x90, 0xE0, // D
                0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
                0xF0, 0x80, 0xF0, 0x80, 0x80, // F
            ],
            video_width: 64,
            video_height: 32,
            opcodes: opcodes,
            opcodes_0: opcodes_0,
            opcodes_8: opcodes_8,
            opcodes_E: opcodes_E,
            opcodes_F: opcodes_F,
            disasm_map: HashMap::new(),
            disasm_opcode: 0,
        }
    }

    pub fn ram(&self) -> *const u8 {
        self.state.ram.as_ptr()
    }

    pub fn framebuffer(&self) -> *const u32 {
        self.state.framebuffer.as_ptr()
    }

    pub fn V(&self) -> *const u8 {
        self.state.V.as_ptr()
    }

    pub fn pc(&self) -> u16 {
        self.state.pc
    }

    pub fn I(&self) -> u16 {
        self.state.I
    }

    pub fn sp(&self) -> u8 {
        self.state.sp
    }

    pub fn delay_timer(&self) -> u8 {
        self.state.delay_timer
    }

    pub fn sound_timer(&self) -> u8 {
        self.state.sound_timer
    }

    pub fn video_height(&self) -> u32 {
        self.video_height
    }

    pub fn video_width(&self) -> u32 {
        self.video_width
    }

    pub fn save_state(&mut self) {
        self.saved_state = self.state.clone();
    }

    pub fn load_state(&mut self) {
        self.state = self.saved_state.clone();
    }

    pub fn disasm_map_serialised(&self) -> JsValue {
        return JsValue::from_serde(&self.disasm_map).unwrap();
    }

    pub fn set_key(&mut self, key: u8, value: u8) {
        match key {
            0..=15 => self.state.keys[key as usize] = value,
            _ => panic!("Writing key out of range"),
        }
    }

    fn read(&mut self, addr: u16) -> u8 {
        match addr {
            0x000..=0xFFF => return self.state.ram[addr as usize],
            _ => panic!("Reading memory out of range"),
        }
    }

    fn write(&mut self, addr: u16, data: u8) {
        match addr {
            0x000..=0xFFF => self.state.ram[addr as usize] = data,
            _ => panic!("Writing memory out of range"),
        }
    }

    pub fn reset(&mut self) {
        self.state.pc = 0x200;
        self.state.opcode = 0;
        self.state.I = 0;
        self.state.sp = 0;
        self.state.delay_timer = 0;
        self.state.sound_timer = 0;

        self.state.ram.iter_mut().for_each(|x| *x = 0);
        self.state.stack.iter_mut().for_each(|x| *x = 0);
        self.state.V.iter_mut().for_each(|x| *x = 0);
        self.state.framebuffer.iter_mut().for_each(|x| *x = 0);
        self.state.keys.iter_mut().for_each(|x| *x = 0);

        for i in 0..80 {
            self.write(i, self.fontset[i as usize]);
        }
    }

    fn load_rom_from_file(&mut self, file_path: &str) {
        self.reset();

        let mut f = File::open(&file_path).expect("Failed to open file");
        //let metadata = f.metadata().expect("Failed to read file metadata");
        f.read(&mut self.state.ram[0x200..])
            .expect("Failed to read file into RAM buffer");
    }

    pub fn load_rom_from_assembler(&mut self, assembler: &Assembler) {
        self.reset();

        self.state.ram[0x200..(0x200 + assembler.binary().len())]
            .clone_from_slice(&assembler.binary());
    }

    pub fn load_rom_from_bytes(&mut self, buffer: &[u8]) {
        self.reset();

        self.state.ram[0x200..(0x200 + buffer.len())].clone_from_slice(&buffer);
    }

    pub fn disassemble(&mut self) {
        let mut done = false;
        let mut i = 0x200;

        self.disasm_opcode = 0;
        self.disasm_map.clear();

        while !done {
            self.disasm_opcode = ((self.read(i) as u16) << 8) | (self.read(i + 1) as u16);
            let disasm: String =
                (self.opcodes[((self.disasm_opcode & 0xF000u16) >> 12) as usize].get_disasm)(self);

            self.disasm_map.insert(i, disasm);
            i += 2;

            if i >= 4096 {
                done = true;
            }
        }
    }

    pub fn clock(&mut self) {
        self.state.opcode =
            ((self.read(self.state.pc) as u16) << 8) | (self.read(self.state.pc + 1) as u16);

        self.state.pc += 2;

        (self.opcodes[((self.state.opcode & 0xF000u16) >> 12) as usize].operation)(self);

        if self.state.delay_timer > 0 {
            self.state.delay_timer -= 1;
        }

        if self.state.sound_timer > 0 {
            self.state.sound_timer -= 1;
        }
    }

    fn opcodes_0_lookup(&mut self) {
        (self.opcodes_0[(self.state.opcode & 0x000Fu16) as usize].operation)(self);
    }

    fn opcodes_0_name_lookup(&mut self) -> String {
        return (self.opcodes_0[(self.disasm_opcode & 0x000Fu16) as usize].get_disasm)(self);
    }

    fn opcodes_8_lookup(&mut self) {
        (self.opcodes_8[(self.state.opcode & 0x000Fu16) as usize].operation)(self);
    }

    fn opcodes_8_name_lookup(&mut self) -> String {
        return (self.opcodes_8[(self.disasm_opcode & 0x000Fu16) as usize].get_disasm)(self);
    }

    fn opcodes_E_lookup(&mut self) {
        (self.opcodes_E[(self.state.opcode & 0x000Fu16) as usize].operation)(self);
    }

    fn opcodes_E_name_lookup(&mut self) -> String {
        return (self.opcodes_E[(self.disasm_opcode & 0x000Fu16) as usize].get_disasm)(self);
    }

    fn opcodes_F_lookup(&mut self) {
        (self.opcodes_F[(self.state.opcode & 0x00FFu16) as usize].operation)(self);
    }

    fn opcodes_F_name_lookup(&mut self) -> String {
        return (self.opcodes_F[(self.disasm_opcode & 0x00FFu16) as usize].get_disasm)(self);
    }

    fn get_args_disasm_nnn(&mut self) -> String {
        let nnn = self.disasm_opcode & 0x0FFFu16;

        return format!("{:X}", nnn);
    }

    fn get_args_disasm_xkk(&mut self) -> String {
        let x = (self.disasm_opcode & 0x0F00u16) >> 8u32;
        let kk = self.disasm_opcode & 0x00FFu16;

        return format!("V{:X}, {:X}", x, kk);
    }

    fn get_args_disasm_xy(&mut self) -> String {
        let x = (self.disasm_opcode & 0x0F00u16) >> 8u32;
        let y = (self.disasm_opcode & 0x00F0u16) >> 4u32;

        return format!("V{:X}, V{:X}", x, y);
    }

    fn get_args_disasm_xyn(&mut self) -> String {
        let x = (self.disasm_opcode & 0x0F00u16) >> 8u32;
        let y = (self.disasm_opcode & 0x00F0u16) >> 4u32;
        let n = self.disasm_opcode & 0x000Fu16;

        return format!("V{:X}, V{:X}, {:X}", x, y, n);
    }

    fn get_args_disasm_x(&mut self) -> String {
        let x = (self.disasm_opcode & 0x0F00u16) >> 8u32;

        return format!("V{:X}", x);
    }

    fn OP_null(&mut self) {
        panic!("Null operator executed!");
    }

    fn OP_0nnn(&mut self) {}

    fn OP_00E0(&mut self) {
        self.state.framebuffer.iter_mut().for_each(|x| *x = 0)
    }

    fn OP_00EE(&mut self) {
        self.state.sp -= 1;
        self.state.pc = self.state.stack[self.state.sp as usize];
    }

    fn OP_1nnn(&mut self) {
        //read the final 12 bits, corresponding to the address to jump to
        let nnn = self.state.opcode & 0x0FFFu16;
        //set the program counter to the address to jump to
        self.state.pc = nnn;
    }

    fn OP_2nnn(&mut self) {
        let nnn = self.state.opcode & 0x0FFFu16;

        self.state.stack[self.state.sp as usize] = self.state.pc;
        self.state.sp += 1;

        self.state.pc = nnn;
    }

    fn OP_3xkk(&mut self) {
        let x = (self.state.opcode & 0x0F00) >> 8u32;
        let kk = self.state.opcode & 0x00FFu16;

        if self.state.V[x as usize] == kk as u8 {
            self.state.pc += 2;
        }
    }

    fn OP_4xkk(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let kk = self.state.opcode & 0x00FFu16;

        if self.state.V[x as usize] != kk as u8 {
            self.state.pc += 2;
        }
    }

    fn OP_5xy0(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        if self.state.V[x as usize] == self.state.V[y as usize] {
            self.state.pc += 2;
        }
    }

    fn OP_6xkk(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let kk = self.state.opcode & 0x00FFu16;

        self.state.V[x as usize] = kk as u8;
    }

    fn OP_7xkk(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let kk = self.state.opcode & 0x00FFu16;

        self.state.V[x as usize] += kk as u8;
    }

    fn OP_9xy0(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        if self.state.V[x as usize] != self.state.V[y as usize] {
            self.state.pc += 2;
        }
    }

    fn OP_Annn(&mut self) {
        let nnn = self.state.opcode & 0x0FFFu16;

        self.state.I = nnn;
    }

    fn OP_Bnnn(&mut self) {
        let nnn = self.state.opcode & 0x0FFFu16;

        self.state.pc = ((self.state.V[0 as usize] as u16) + (nnn)) as u16;
    }

    fn OP_Cxkk(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let kk = self.state.opcode & 0x00FFu16;

        let mut buf = [0u8; 1];
        getrandom::getrandom(&mut buf).expect("random number generation failed");

        self.state.V[x as usize] = (buf[0] as u16 & kk) as u8;
    }

    fn OP_Dxyn(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;
        let height = self.state.opcode & 0x000Fu16;

        let x_pos = self.state.V[x as usize] as u32;
        let y_pos = self.state.V[y as usize] as u32;

        self.state.V[0xF] = 0;

        for row in 0..height {
            let sprite_byte = self.read(self.state.I + row);

            for col in 0..8 {
                let sprite_pixel = sprite_byte & (0x80 >> col);
                //utils::log!("y pos: {}, row: {}, width: {}, x_pos: {}, col: {}", y_pos, row, self.video_width, x_pos, col);
                let index = ((y_pos + row as u32) % self.video_height) * self.video_width
                    + ((x_pos + col) % self.video_width);
                let screen_pixel = &mut self.state.framebuffer[index as usize];

                if sprite_pixel > 0 {
                    if *screen_pixel == 0xFFFFFFFF {
                        self.state.V[0xF] = 1;
                    }

                    *screen_pixel ^= 0xFFFFFFFF;
                }
            }
        }
    }

    fn OP_8xy0(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        self.state.V[x as usize] = self.state.V[y as usize];
    }

    fn OP_8xy1(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        self.state.V[x as usize] |= self.state.V[y as usize];
    }

    fn OP_8xy2(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        self.state.V[x as usize] &= self.state.V[y as usize];
    }

    fn OP_8xy3(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        self.state.V[x as usize] ^= self.state.V[y as usize];
    }

    fn OP_8xy4(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        let sum: u16 = self.state.V[x as usize] as u16 + self.state.V[y as usize] as u16;
        if sum > 255 {
            self.state.V[0xF] = 1;
        } else {
            self.state.V[0xF] = 0;
        }

        self.state.V[x as usize] = (sum & 0x00FFu16) as u8;
    }

    fn OP_8xy5(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        if self.state.V[x as usize] > self.state.V[y as usize] {
            self.state.V[0xF] = 1;
        } else {
            self.state.V[0xF] = 0;
        }

        self.state.V[x as usize] -= self.state.V[y as usize];
    }

    fn OP_8xy6(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.V[0xF] = self.state.V[x as usize] & 0x1;

        self.state.V[x as usize] >>= 1;
    }

    fn OP_8xy7(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let y = (self.state.opcode & 0x00F0u16) >> 4u32;

        if self.state.V[y as usize] > self.state.V[x as usize] {
            self.state.V[0xF] = 1;
        } else {
            self.state.V[0xF] = 0;
        }

        self.state.V[x as usize] = self.state.V[y as usize] - self.state.V[x as usize];
    }

    fn OP_8xyE(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.V[0xF] = (self.state.V[x as usize] & 0x80) >> 7u32;

        self.state.V[x as usize] <<= 1;
    }

    fn OP_Ex9E(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let key = self.state.V[x as usize];

        if self.state.keys[key as usize] > 0 {
            self.state.pc += 2;
        }
    }

    fn OP_ExA1(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let key = self.state.V[x as usize];

        if self.state.keys[key as usize] == 0 {
            self.state.pc += 2;
        }
    }

    fn OP_Fx07(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.V[x as usize] = self.state.delay_timer;
    }

    fn OP_Fx0A(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        let mut key_pressed = false;
        for (idx, element) in self.state.keys.iter().enumerate() {
            if self.state.keys[idx as usize] > 0 {
                self.state.V[x as usize] = idx as u8;
                key_pressed = true;
                utils::log!("{}", self.state.keys[idx as usize]);
                break;
            }
        }

        if !key_pressed {
            self.state.pc -= 2;
        }
    }

    fn OP_Fx15(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.delay_timer = self.state.V[x as usize]
    }

    fn OP_Fx18(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.sound_timer = self.state.V[x as usize]
    }

    fn OP_Fx1E(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.I += self.state.V[x as usize] as u16;
    }

    fn OP_Fx29(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        self.state.I = (self.state.V[x as usize] * 5) as u16;
    }

    fn OP_Fx33(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;
        let mut val = self.state.V[x as usize];

        self.write(self.state.I + 2, val % 10);
        val /= 10;

        self.write(self.state.I + 1, val % 10);
        val /= 10;

        self.write(self.state.I, val % 10);
    }

    fn OP_Fx55(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        for i in 0..=x {
            self.write(self.state.I + i, self.state.V[i as usize]);
        }

        self.state.I += x + 1;
    }

    fn OP_Fx65(&mut self) {
        let x = (self.state.opcode & 0x0F00u16) >> 8u32;

        for i in 0..=x {
            self.state.V[i as usize] = self.read(self.state.I + i);
        }

        self.state.I += x + 1;
    }
}

#[cfg(test)]
mod tests {
    use super::Chip8;

    #[test]
    pub fn test_00E0() {
        let mut c8 = Chip8::new();
        let clone = c8.framebuffer().clone();

        //c8.framebuffer.iter_mut().for_each(|x| *x = 15);

        c8.OP_00E0();

        assert_eq!(clone, c8.framebuffer());
    }

    #[test]
    pub fn test_00EE() {
        let mut c8 = Chip8::new();

        let code: [u8; 4] = [0x22, 0x02, 0x00, 0xEE]; //CALL 202, RET
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.sp(), 0);
        assert_eq!(c8.pc(), 0x202);
    }

    #[test]
    pub fn test_1nnn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x12, 0x8C]; //JP, 28C
        c8.load_rom_from_bytes(&code);
        c8.clock();
        assert_eq!(c8.pc(), 0x28C);
    }

    #[test]
    pub fn test_2nnn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x24, 0x00]; //CALL 400
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.sp(), 1);
        assert_eq!(c8.pc(), 0x400);
        assert_eq!(c8.state.stack[(c8.sp() - 1) as usize], 0x202);
    }

    #[test]
    pub fn test_3xkk() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x35, 0x0]; //SE V5, 0
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x204);
    }

    #[test]
    pub fn test_4xkk() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x45, 0x07]; //SNE V5, 7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x204);
    }

    #[test]
    pub fn test_5xy0() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x55, 0x70]; //SE V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x204);
    }

    #[test]
    pub fn test_6xkk() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x63, 0x65]; //LD V3, 65
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x3], 0x65);
    }

    #[test]
    pub fn test_7xkk() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x73, 0x20]; //ADD V3, 20
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x3], 0x20);
    }

    #[test]
    pub fn test_8xy0() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x70]; //LD V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0);
    }

    #[test]
    pub fn test_8xy1() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x71]; //OR V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 | 0x0);
    }

    #[test]
    pub fn test_8xy2() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x72]; //AND V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 & 0x0);
    }

    #[test]
    pub fn test_8xy3() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x73]; //XOR V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 ^ 0x0);
    }

    #[test]
    pub fn test_8xy4() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x74]; //ADD V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 + 0x0);
    }

    #[test]
    pub fn test_8xy5() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x75]; //SUB V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 - 0x0);
    }

    #[test]
    pub fn test_8xy6() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x06]; //SHR V5
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 >> 1);
    }

    #[test]
    pub fn test_8xy7() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x77]; //SHR V5
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 - 0x0);
    }

    #[test]
    pub fn test_8xyE() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x7E]; //SHL V5
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0x5], 0x0 << 1);
    }

    #[test]
    pub fn test_9xy0() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x85, 0x70]; //SNE V5, V7
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x202);
    }

    #[test]
    pub fn test_Annn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xA5, 0x70]; //LD I, 570
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.I(), 0x570);
    }

    #[test]
    pub fn test_Bnnn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xB5, 0x70]; //JP V0, 570
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x570);
    }

    #[test]
    pub fn test_Cxkk() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xC0, 0x00]; //RND V0, 00
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.V[0], 0);
    }

    #[test]
    pub fn test_Dxyn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xD0, 0x01]; //DRW V0, V0, 1
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.state.framebuffer[0], 0xFFFFFFFF);
    }

    #[test]
    pub fn test_Ex9E() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xE0, 0x9E]; //SKP V0
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x202);
    }

    #[test]
    pub fn test_ExA1() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xE0, 0xA1]; //SKNP V0
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.pc(), 0x204);
    }

    #[test]
    pub fn test_Fx07() {
        let mut c8 = Chip8::new();
        let code: [u8; 6] = [0x60, 0x05, 0xF0, 0x15, 0xF0, 0x07]; //LD V0, 5; LD DT, V0; LD V0, DT
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();
        c8.clock();

        assert_eq!(c8.state.V[0], 0x4);
    }

    #[test]
    pub fn test_Fx0A() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xF0, 0x0A]; //LD V0, K
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();
        c8.clock();

        assert_eq!(c8.pc(), 0x200);
    }

    #[test]
    pub fn test_Fx15() {
        let mut c8 = Chip8::new();
        let code: [u8; 4] = [0x60, 0x05, 0xF0, 0x15]; //LD V0, 5; LD DT, V0;
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.delay_timer(), 0x4);
    }

    #[test]
    pub fn test_Fx18() {
        let mut c8 = Chip8::new();
        let code: [u8; 4] = [0x60, 0x05, 0xF0, 0x18]; //LD V0, 5; LD ST, V0;
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.sound_timer(), 0x4);
    }

    #[test]
    pub fn test_Fx1E() {
        let mut c8 = Chip8::new();
        let code: [u8; 4] = [0x60, 0x05, 0xF0, 0x1E]; //LD V0, 5; ADD I, Vx
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.I(), 5);
    }

    #[test]
    pub fn test_Fx29() {
        let mut c8 = Chip8::new();
        let code: [u8; 4] = [0x60, 0x05, 0xF0, 0x29]; //LD V0, 5; LD F, V0
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.I(), 25);
    }

    #[test]
    pub fn test_Fx33() {
        let mut c8 = Chip8::new();
        let code: [u8; 4] = [0x60, 0x80, 0xF0, 0x33]; //LD V0, 80; LD B, V0
        c8.load_rom_from_bytes(&code);
        c8.clock();
        c8.clock();

        assert_eq!(c8.read(c8.I()), 1);
        assert_eq!(c8.read(c8.I() + 1), 2);
        assert_eq!(c8.read(c8.I() + 2), 8);
    }

    #[test]
    pub fn test_Fx55() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xF8, 0x55]; //LD [I], V8
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.I(), 0x9);
    }

    #[test]
    pub fn test_Fx65() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0xF8, 0x65]; //LD V8, [I]
        c8.load_rom_from_bytes(&code);
        c8.clock();

        assert_eq!(c8.I(), 0x9);
    }

    #[test]
    pub fn test_disasm_1nnn() {
        let mut c8 = Chip8::new();
        let code: [u8; 2] = [0x15, 0x5D]; //JP 55D
        c8.load_rom_from_bytes(&code);
        c8.disassemble();

        assert_eq!("JP 55D", c8.disasm_map.get(&0x200).unwrap());
    }

    #[test]
    pub fn test_disasm_nnnk() {
        let mut c8 = Chip8::new();
        c8.disasm_opcode = 0xA6AD;

        assert_eq!("6AD", c8.get_args_disasm_nnn());
    }

    #[test]
    pub fn test_disasm_xkk() {
        let mut c8 = Chip8::new();
        c8.disasm_opcode = 0x622C;

        assert_eq!("V2, 2C", c8.get_args_disasm_xkk());
    }

    #[test]
    pub fn test_disasm() {
        let mut c8 = Chip8::new();
        c8.disasm_opcode = 0x147C;

        assert_eq!(
            "JP 47C",
            (c8.opcodes[((c8.disasm_opcode & 0xF000u16) >> 12) as usize].get_disasm)(&mut c8)
        );

        c8.disasm_opcode = 0x00E0;

        assert_eq!(
            "CLS",
            (c8.opcodes[((c8.disasm_opcode & 0xF000u16) >> 12) as usize].get_disasm)(&mut c8)
        );

        c8.disasm_opcode = 0x35D0;

        assert_eq!(
            "SE V5, D0",
            (c8.opcodes[((c8.disasm_opcode & 0xF000u16) >> 12) as usize].get_disasm)(&mut c8)
        );

        c8.disasm_opcode = 0xF955;

        assert_eq!(
            "LD [I], V9",
            (c8.opcodes[((c8.disasm_opcode & 0xF000u16) >> 12) as usize].get_disasm)(&mut c8)
        );
    }
}
