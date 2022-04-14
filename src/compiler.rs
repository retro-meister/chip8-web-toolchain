use crate::lexer::TokenType::*;
use crate::lexer::*;
use crate::utils;

use wasm_bindgen::prelude::*;

use std::array::IntoIter;
use std::collections::HashMap;
use std::fmt;
use std::fmt::Debug;
use std::iter::FromIterator;

use num_enum::TryFromPrimitive;
use std::convert::TryFrom;

use CompileRuleType::*;
use Opcode::*;

type CompileFn = fn(&mut Compiler, bool);

#[derive(PartialEq, PartialOrd, TryFromPrimitive)]
#[repr(u8)]
pub enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Term,   /* + and - */
    Factor, /* * and / */
    Primary,
}

#[derive(Clone)]
pub struct Variable {
    name: String,
    reg_index: u16,
    scope_depth: u16,
}

impl Variable {
    pub fn new(name: String, reg_index: u16, scope_depth: u16) -> Variable {
        Variable {
            name,
            reg_index,
            scope_depth,
        }
    }
}

pub struct Function {
    start_addr: u16,
    args: Vec<String>,
}

impl Function {
    pub fn new(start_addr: u16) -> Function {
        Function {
            start_addr,
            args: Vec::new(),
        }
    }
}

pub enum CompileRuleType {
    Prefix { prefix: CompileFn },
    Infix { infix: CompileFn },
    PrefixAndInfix { prefix: CompileFn, infix: CompileFn },
    Neither,
}

pub struct CompileRule {
    precedence: Precedence,
    rule_type: CompileRuleType,
}

impl CompileRule {
    pub fn new(precedence: Precedence, rule_type: CompileRuleType) -> CompileRule {
        CompileRule {
            precedence: precedence,
            rule_type: rule_type,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Opcode {
    LDRegByte(u16, u16),
    LDRegReg(u16, u16),
    AddRegReg(u16, u16),
    SubRegReg(u16, u16),
    SERegReg(u16, u16),
    SNERegReg(u16, u16),
    LDFReg(u16),
    LDIReg(u16),
    LDRegI(u16),
    LDDTReg(u16),
    LDRegDT(u16),
    LDSTReg(u16),
    LDRegKey(u16),
    LDIAddr(u16),
    RNDRegByte(u16, u16),
    DRWRegRegNibble(u16, u16, u16),
    JP(u16),
    CALL(u16),
    RET,
}

/*impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LDRegByte(reg, byte) => write!(f, "LD V{}, {}", reg, byte),
            LDRegReg(reg1, reg2) => write!(f, "LD V{}, V{}", reg1, reg2),
            AddRegReg(reg1, reg2) => write!(f, "ADD V{}, V{}", reg1, reg2),
            SubRegReg(reg1, reg2) => write!(f, "SUB V{}, V{}", reg1, reg2),
            SERegReg(reg1, reg2) => write!(f, "SE V{}, V{}", reg1, reg2),
            SNERegReg(reg1, reg2) => write!(f, "SNE V{}, V{}", reg1, reg2),
            LDFReg(reg) => write!(f, "LD F, V{}", reg),
            LDIReg(reg) => write!(f, "LD [I], V{}", reg),
            LDRegI(reg) => write!(f, "LD V{}, I[]", reg),
            JP(addr) => write!(f, "JP {}", addr),
            CALL(addr) => write!(f, "CALL {}", addr),
            RET => write!(f, "RET"),
        }
    }
}*/

impl fmt::Display for Opcode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

pub fn asm_bytes_len(len: usize) -> u16 {
    (len as u16 * 2) + 0x200
}

#[wasm_bindgen]
pub struct Compiler {
    tokens: Vec<Token>,
    current: usize,
    previous: usize,
    reg_stack_top: u16,
    scope_depth: u16,
    variables: Vec<Variable>,
    functions: HashMap<String, Function>,
    asm: Vec<Opcode>,
    ram_line_map: HashMap<u16, u32>,
}

#[wasm_bindgen]
impl Compiler {
    pub fn new_from_lexer(lexer: &Lexer) -> Compiler {
        Compiler {
            tokens: lexer.tokens().clone(),
            current: 0,
            previous: 0,
            reg_stack_top: 0,
            scope_depth: 0,
            variables: Vec::new(),
            functions: HashMap::new(),
            asm: Vec::new(),
            ram_line_map: HashMap::new(),
        }
    }

    pub fn ram_line_map_serialised(&self) -> JsValue {
        return JsValue::from_serde(&self.ram_line_map).unwrap();
    }

    fn get_rule(&self, token: &Token) -> CompileRule {
        match token.token_type() {
            Plus | Minus => CompileRule::new(
                Precedence::Term,
                Infix {
                    infix: Compiler::binary,
                },
            ),
            Equals | Semicolon | RightParen | Comma => CompileRule::new(Precedence::None, Neither),
            Number(_) => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::number,
                },
            ),
            Identifier(_) => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::variable,
                },
            ),
            EqualsEquals | NotEquals => CompileRule::new(
                Precedence::Equality,
                Infix {
                    infix: Compiler::binary,
                },
            ),
            And => CompileRule::new(
                Precedence::And,
                Infix {
                    infix: Compiler::and,
                },
            ),
            Or => CompileRule::new(
                Precedence::Or,
                Infix {
                    infix: Compiler::or,
                },
            ),
            DT => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::DT,
                },
            ),
            ST => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::ST,
                },
            ),
            I => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::I,
                },
            ),
            Rand => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::rand,
                },
            ),
            Key => CompileRule::new(
                Precedence::None,
                Prefix {
                    prefix: Compiler::key,
                },
            ),
            _ => panic!(
                "cant find rule for {} in get_rule()",
                token.token_type().to_string()
            ),
        }
    }

    fn compile_precedence(&mut self, precedence: Precedence) {
        self.advance();
        let assign_allowed = precedence <= Precedence::Assignment;

        let prev = self.tokens[self.previous].clone();

        match self.get_rule(&prev).rule_type {
            Prefix { prefix } => prefix(self, assign_allowed),
            PrefixAndInfix { prefix, .. } => prefix(self, assign_allowed),
            _ => panic!(
                "no prefix rule in compile_precedence() for {}",
                prev.token_type()
            ),
        }

        while precedence <= self.get_rule(&self.tokens[self.current]).precedence {
            self.advance();
            match self.get_rule(&self.tokens[self.previous]).rule_type {
                Infix { infix } => infix(self, assign_allowed),
                PrefixAndInfix { prefix, infix } => infix(self, assign_allowed),
                _ => (),
            }
        }
    }

    fn emit(&mut self, opcode: Opcode) {
        let line = self.tokens[self.previous].line;
        self.ram_line_map
            .insert(asm_bytes_len(self.asm.len()), line);
        self.asm.push(opcode);
    }

    pub fn lookup_variable_register(&self, name: String) -> Option<u16> {
        for var in self.variables.iter().rev() {
            if var.name == name {
                return Some(var.reg_index);
            }
        }
        return None;
    }

    pub fn clear_current_scope(&mut self) {
        for i in (0..self.variables.len()).rev() {
            if self.variables[i].scope_depth == self.scope_depth {
                self.variables.remove(i);
                self.reg_stack_top -= 1;
            }
        }
    }

    pub fn stringify_asm(&self) -> String {
        self.asm
            .iter()
            .map(|asm| asm.to_string())
            .collect::<Vec<String>>()
            .join("\n")
    }

    pub fn inc_reg_stack_top(&mut self) {
        self.reg_stack_top += 1;
    }

    pub fn dec_reg_stack_top(&mut self) {
        self.reg_stack_top -= 1;
    }

    fn peek_reg_stack(&self, depth: u16) -> u16 {
        self.reg_stack_top - 1 - depth
    }

    fn advance(&mut self) {
        self.previous = self.current;

        self.current += 1;
    }

    fn check(&self, token: TokenType) -> bool {
        self.tokens[self.current].token_type() == token
    }

    fn consume(&mut self, token: TokenType) {
        let cur = self.tokens[self.current].clone().token_type();
        match cur == token {
            true => self.advance(),
            false => panic!(
                "token {} didn't match in consume(), found {} instead",
                token.to_string(),
                cur.to_string()
            ),
        }
    }

    pub fn compile(&mut self) {
        while !self.check(EndOfFile) {
            //self.advance();
            self.declaration();
        }
    }

    pub fn declaration(&mut self) {
        if self.check(Fn) {
            self.advance();
            self.fn_declaration();
        } else if self.check(Var) {
            self.advance();
            self.var_declaration();
        } else {
            self.statement();
        }
    }

    pub fn fn_declaration(&mut self) {
        let mut cur_arg_assigned_reg = 0;
        let mut has_args = false;
        let mut fn_name = String::from("");
        match self.tokens[self.current].clone().token_type {
            Identifier(name) => {
                self.advance();
                fn_name = name.clone();
                self.functions.insert(
                    name.clone(),
                    Function::new(asm_bytes_len(self.asm.len()) + 2),
                );
            }
            _ => panic!("identifier name must follow fn keyword"),
        }

        self.consume(LeftParen);
        if !self.check(RightParen) {
            self.advance();
            has_args = true;
            match self.tokens[self.previous].clone().token_type() {
                Identifier(name) => {
                    self.functions
                        .get_mut(&fn_name)
                        .expect(&format!("function {} not found", &fn_name))
                        .args
                        .push(name.clone());
                    self.variables.push(Variable::new(
                        name.clone(),
                        cur_arg_assigned_reg,
                        self.scope_depth,
                    ));
                }
                _ => panic!("non-identifier matched while parsing function args"),
            }
            while self.check(Comma) {
                cur_arg_assigned_reg += 1;
                self.advance();
                self.advance();
                match self.tokens[self.previous].clone().token_type() {
                    Identifier(name) => {
                        self.functions
                            .get_mut(&fn_name)
                            .expect(&format!("function {} not found", &fn_name))
                            .args
                            .push(name.clone());
                        self.variables.push(Variable::new(
                            name.clone(),
                            cur_arg_assigned_reg,
                            self.scope_depth,
                        ));
                    }
                    _ => panic!("non-identifier matched while parsing function args"),
                }
            }
        }

        self.consume(RightParen);
        self.consume(LeftBrace);

        self.scope_depth += 1;

        let reg_stack_top_backup = self.reg_stack_top;
        match has_args {
            true => self.reg_stack_top = cur_arg_assigned_reg + 1,
            false => self.reg_stack_top = cur_arg_assigned_reg,
        }

        let jp_over_fn_asm_index = self.asm.len();
        self.emit(JP(0));
        self.block();
        self.pop_frame();

        self.asm[jp_over_fn_asm_index] = JP(asm_bytes_len(self.asm.len()));

        self.clear_current_scope();
        self.scope_depth -= 1;

        self.reg_stack_top = reg_stack_top_backup;
    }

    pub fn push_frame(&mut self) {
        self.emit(LDFReg(0xD));
        self.emit(LDIReg(0xD));
        self.emit(LDRegByte(0xE, 3));
        self.emit(AddRegReg(0xD, 0xE));
    }

    pub fn pop_frame(&mut self) {
        self.emit(LDRegByte(0xE, 3));
        self.emit(SubRegReg(0xD, 0xE));
        //self.emit(LDRegReg(0xF, self.reg_stack_top));
        self.emit(LDFReg(0xD));
        self.emit(LDRegI(0xD));
        //self.emit(LDRegReg(self.reg_stack_top, 0xF));
        self.emit(RET);
    }

    pub fn var_declaration(&mut self) {
        match self.tokens[self.current].clone().token_type() {
            Identifier(name) => {
                self.advance();
                self.variables.push(Variable::new(
                    name.clone(),
                    self.reg_stack_top,
                    self.scope_depth,
                ));
                match self.tokens[self.current].clone().token_type() {
                    Equals => {
                        self.advance();
                        self.expression()
                    }
                    _ => panic!("initialiser must be present in variable declaration"),
                }
            }
            _ => panic!("identifier must follow after var keyword"),
        }

        if self.check(Equals) {
            self.advance();
            self.expression();
        }

        self.consume(Semicolon);
    }

    fn statement(&mut self) {
        if self.check(LeftBrace) {
            self.advance();
            self.scope_depth += 1;
            self.block();
            //decrement reg_stack_top until scope_depth of variable changes
            self.clear_current_scope();
            self.scope_depth -= 1;
        } else if self.check(If) {
            self.advance();
            self.if_statement();
        } else if self.check(While) {
            self.advance();
            self.while_statement();
        } else if self.check(Draw) {
            self.advance();
            self.draw_statement();
        } else {
            self.expression_statement();
        }
    }

    fn block(&mut self) {
        while !self.check(RightBrace) && !self.check(EndOfFile) {
            self.declaration();
        }

        self.consume(RightBrace);
    }

    fn if_statement(&mut self) {
        self.consume(LeftParen);
        self.expression();
        self.consume(RightParen);

        let jp_asm_index = self.asm.len();
        self.emit(JP(0));
        self.statement();

        if self.check(Else) {
            self.asm[jp_asm_index] = JP(asm_bytes_len(self.asm.len()) + 2);
            self.advance();
            let jp_asm_index = self.asm.len();
            self.emit(JP(0));
            self.statement();
            self.asm[jp_asm_index] = JP(asm_bytes_len(self.asm.len()));
        } else {
            self.asm[jp_asm_index] = JP(asm_bytes_len(self.asm.len()));
        }
    }

    fn while_statement(&mut self) {
        let while_start = asm_bytes_len(self.asm.len());

        self.consume(LeftParen);
        self.expression();
        self.consume(RightParen);

        //jump to after loop if condition not met
        let jp_condition_not_met_asm_index = self.asm.len();
        self.emit(JP(0));
        self.statement();

        //jump back to start of while loop to retest condition
        let jp_loop_asm = self.asm.len();
        self.emit(JP(0));
        self.asm[jp_loop_asm] = JP(while_start as u16);

        self.asm[jp_condition_not_met_asm_index] = JP(asm_bytes_len(self.asm.len()));
    }

    fn draw_statement(&mut self) {
        self.consume(LeftParen);
        self.expression();
        self.consume(Comma);
        self.expression();
        self.consume(Comma);
        match self.tokens[self.current].token_type() {
            Number(num) => {
                self.advance();
                self.consume(RightParen);
                self.emit(DRWRegRegNibble(self.peek_reg_stack(1), self.peek_reg_stack(0), num.clone()));
                self.dec_reg_stack_top();
                self.dec_reg_stack_top();
            }
            _ => panic!("number literal param must be passed to rand() to AND result with (variable/expression cannot be used)")
        }
        self.consume(Semicolon);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.consume(Semicolon);
        self.dec_reg_stack_top();
    }

    fn expression(&mut self) {
        self.compile_precedence(Precedence::Assignment);
    }

    fn number(&mut self, assign_allowed: bool) {
        //self.inc_reg_stack_top();
        let prev = self.tokens[self.previous].clone().token_type();
        match prev {
            Number(num) => self.emit(LDRegByte(self.reg_stack_top, num.clone())),
            _ => panic!("non number matched in number()"),
        }
        self.inc_reg_stack_top();
    }

    fn variable(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            Identifier(name) => match cur {
                Equals => {
                    self.advance();
                    self.expression();
                    self.emit(LDRegReg(
                        self.lookup_variable_register(name.clone())
                            .expect(format!("variable {} not found", &name.clone()).as_str()),
                        self.peek_reg_stack(0),
                    ));
                    self.dec_reg_stack_top();
                }
                LeftParen => {
                    //maybe instead call parse precedence here and go thru that way??
                    self.advance();

                    self.push_frame();

                    if !self.check(RightParen) {
                        self.expression();
                        while self.check(Comma) {
                            self.advance();
                            self.expression();
                        }
                    }

                    let num_args = self
                        .functions
                        .get(&name.clone())
                        .expect(format!("function {} not found", &name.clone()).as_str())
                        .args
                        .len();
                    for i in 0..num_args {
                        self.emit(LDRegReg(
                            i as u16,
                            (self.reg_stack_top - num_args as u16) + i as u16,
                        ))
                    }

                    self.reg_stack_top -= num_args as u16;

                    self.consume(RightParen);

                    self.emit(CALL(self.functions.get(&name.clone()).unwrap().start_addr));
                }
                _ => {
                    self.emit(LDRegReg(
                        self.reg_stack_top,
                        self.lookup_variable_register(name.clone())
                            .expect(format!("variable {} not found", &name.clone()).as_str()),
                    ));
                }
            },
            _ => {
                panic!("non identifier matched in variable()");
            }
        }

        self.inc_reg_stack_top();
    }

    fn DT(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            DT => match cur {
                Equals => {
                    self.advance();
                    self.expression();
                    self.emit(LDDTReg(self.peek_reg_stack(0)));
                }
                _ => {
                    self.emit(LDRegDT(self.reg_stack_top));
                    self.inc_reg_stack_top();
                }
            },
            _ => {
                panic!("non DT matched in DT()");
            }
        }
    }

    fn ST(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            ST => match cur {
                Equals => {
                    self.advance();
                    self.expression();
                    self.emit(LDSTReg(self.peek_reg_stack(0)));
                }
                _ => panic!("equals must follow ST as it can only be assigned to, not read"),
            },
            _ => {
                panic!("non ST matched in ST()");
            }
        }
    }

    fn I(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            I => match cur {
                Equals => {
                    self.advance();
                    match self.tokens[self.current].token_type() {
                        Number(num) => {
                            self.advance();
                            self.emit(LDIAddr(num.clone()));
                            self.inc_reg_stack_top();
                        }
                        _ => panic!("I must be assigned to number literal (variable/expression cannot be used)")
                    }
                }
                _ => panic!("equals must follow I as it can only be assigned to, not read"),
            },
            _ => {
                panic!("non I matched in I()");
            }
        }
    }

    fn rand(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            Rand => match cur {
                LeftParen => {
                    self.consume(LeftParen);
                    match self.tokens[self.current].token_type() {
                        Number(num) => {
                            self.advance();
                            self.consume(RightParen);
                            self.emit(RNDRegByte(self.reg_stack_top, num.clone()));
                            self.inc_reg_stack_top();
                        }
                        _ => panic!("number literal param must be passed to rand() to AND result with (variable/expression cannot be used)")
                    }
                }
                _ => panic!("number literal param must be passed to rand() to AND result with (variable/expression cannot be used)")
            },
            _ => {
                panic!("non rand matched in rand()");
            }
        }
    }

    fn key(&mut self, assign_allowed: bool) {
        let prev = self.tokens[self.previous].clone().token_type();
        let cur = self.tokens[self.current].clone().token_type();

        match prev {
            Key => match cur {
                LeftParen => {
                    self.consume(LeftParen);
                    self.consume(RightParen);
                    self.emit(LDRegKey(self.reg_stack_top));
                    self.inc_reg_stack_top();
                }
                _ => panic!("expect () after key"),
            },
            _ => {
                panic!("non rand matched in rand()");
            }
        }
    }

    fn binary(&mut self, assign_allowed: bool) {
        let binop_type = self.tokens[self.previous].clone().token_type;
        let next_prec =
            Precedence::try_from(self.get_rule(&self.tokens[self.previous]).precedence as u8 + 1)
                .unwrap();
        self.compile_precedence(next_prec);

        match binop_type {
            Plus => {
                self.emit(AddRegReg(self.peek_reg_stack(1), self.peek_reg_stack(0)));
                self.dec_reg_stack_top();
            }
            Minus => {
                self.emit(SubRegReg(self.peek_reg_stack(1), self.peek_reg_stack(0)));
                self.dec_reg_stack_top();
            }
            EqualsEquals => {
                self.emit(SERegReg(self.peek_reg_stack(1), self.peek_reg_stack(0)));
                self.dec_reg_stack_top();
                self.dec_reg_stack_top();
            }
            NotEquals => {
                self.emit(SNERegReg(self.peek_reg_stack(1), self.peek_reg_stack(0)));
                self.dec_reg_stack_top();
                self.dec_reg_stack_top();
            }
            _ => panic!(
                "non binary op {} found in binary()",
                self.tokens[self.previous].token_type.to_string()
            ),
        }
    }

    fn or(&mut self, assign_allowed: bool) {
        let jp_condition_not_met_asm_index = self.asm.len();
        self.emit(JP(0));
        let jp_condition_met_asm_index = self.asm.len();
        self.emit(JP(0));

        self.asm[jp_condition_not_met_asm_index] = JP(asm_bytes_len(self.asm.len()));
        self.compile_precedence(Precedence::Or);
        self.asm[jp_condition_met_asm_index] = JP(asm_bytes_len(self.asm.len()) + 2);
    }

    fn and(&mut self, assign_allowed: bool) {
        let jp_asm_index = self.asm.len();
        self.emit(JP(0));

        self.compile_precedence(Precedence::And);

        self.asm[jp_asm_index] = JP(asm_bytes_len(self.asm.len()));
    }
}

impl Compiler {
    pub fn asm(&self) -> &Vec<Opcode> {
        &self.asm
    }
}

#[cfg(test)]
mod tests {
    use super::Compiler;
    use super::*;

    #[test]
    pub fn test_check() {
        let mut l = Lexer::new("var test 123 55");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        assert!(c.check(Var));
    }

    #[test]
    pub fn test_number() {
        let mut l = Lexer::new("10; 5;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        let mut l = Lexer::new("12 + 3 + 7 + 2;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        assert_eq!(c.reg_stack_top, 0);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 12),
                LDRegByte(1, 3),
                AddRegReg(0, 1),
                LDRegByte(1, 7),
                AddRegReg(0, 1),
                LDRegByte(1, 2),
                AddRegReg(0, 1),
            ]
        ));

        assert_eq!(c.reg_stack_top, 0);
    }

    #[test]
    pub fn test_sub() {
        let mut l = Lexer::new("9 - 7;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![LDRegByte(0, 9), LDRegByte(1, 7), SubRegReg(0, 1)]
        ));
        assert_eq!(c.reg_stack_top, 0);
    }

    #[test]
    pub fn test_variable() {
        let mut l = Lexer::new("var a = 3; a;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![LDRegByte(0, 3), LDRegReg(1, 0)]
        ));
        assert_eq!(c.reg_stack_top, 1);
    }

    #[test]
    pub fn test_variable_assignment() {
        let mut l = Lexer::new("var a = 1; a + 4; var b = 2; var c = b + a; c = a;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            //vec![LDRegByte(0, 3), LDRegByte(1, 10), LDRegReg(0, 1)]
            vec![
                LDRegByte(0, 1),
                LDRegReg(1, 0),
                LDRegByte(2, 4),
                AddRegReg(1, 2),
                LDRegByte(1, 2),
                LDRegReg(2, 1),
                LDRegReg(3, 0),
                AddRegReg(2, 3),
                LDRegReg(3, 0),
                LDRegReg(2, 3)
            ]
        ));
        assert_eq!(c.reg_stack_top, 3);
    }

    #[test]
    pub fn test_lexical_scope() {
        let mut l = Lexer::new("var a = 1; { var b = 4; } var c = 7;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            //vec![LDRegByte(0, 3), LDRegByte(1, 10), LDRegReg(0, 1)]
            vec![LDRegByte(0, 1), LDRegByte(1, 4), LDRegByte(1, 7),]
        ));
        assert_eq!(c.reg_stack_top, 2);
    }

    #[test]
    pub fn test_if() {
        let mut l = Lexer::new("if (1+3 == 4) { 10; } 5;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            //vec![LDRegByte(0, 3), LDRegByte(1, 10), LDRegReg(0, 1)]
            vec![
                LDRegByte(0, 1),
                LDRegByte(1, 3),
                AddRegReg(0, 1),
                LDRegByte(1, 4),
                SERegReg(0, 1),
                JP(0x20E),
                LDRegByte(0, 10),
                LDRegByte(0, 5)
            ]
        ));
    }

    #[test]
    pub fn test_if_else() {
        let mut l = Lexer::new("var a = 0; if (1 == 2) a = 5; else a = 9;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 0),
                LDRegByte(1, 1),
                LDRegByte(2, 2),
                SERegReg(1, 2),
                JP(0x210),
                LDRegByte(1, 5),
                LDRegReg(0, 1),
                JP(0x214),
                LDRegByte(1, 9),
                LDRegReg(0, 1)
            ]
        ));
    }

    #[test]
    pub fn test_and() {
        let mut l = Lexer::new("if (2 == 2 and 4 == 4) 5; else 9;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 2),
                LDRegByte(1, 2),
                SERegReg(0, 1),
                JP(0x20E),
                LDRegByte(0, 4),
                LDRegByte(1, 4),
                SERegReg(0, 1),
                JP(0x214),
                LDRegByte(0, 5),
                JP(0x216),
                LDRegByte(0, 9)
            ]
        ));
    }

    #[test]
    pub fn test_not_equal() {
        let mut l = Lexer::new("if (1 != 5) 3;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();
        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 1),
                LDRegByte(1, 5),
                SNERegReg(0, 1),
                JP(0x20A),
                LDRegByte(0, 3),
            ]
        ));
    }

    #[test]
    pub fn test_or() {
        let mut l = Lexer::new("if (1 != 1 or 3 == 3) 8; else 5;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 1),
                LDRegByte(1, 1),
                SNERegReg(0, 1),
                JP(0x20A),
                JP(0x212),
                LDRegByte(0, 3),
                LDRegByte(1, 3),
                SERegReg(0, 1),
                JP(0x216),
                LDRegByte(0, 8),
                JP(0x218),
                LDRegByte(0, 5),
            ]
        ));
    }

    #[test]
    pub fn test_while() {
        let mut l = Lexer::new("var a = 255; while (a != 0) { a = a - 1; }");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 255),
                LDRegReg(1, 0),
                LDRegByte(2, 0),
                SNERegReg(1, 2),
                JP(0x214),
                LDRegReg(1, 0),
                LDRegByte(2, 1),
                SubRegReg(1, 2),
                LDRegReg(0, 1),
                JP(0x202),
            ]
        ));
    }

    #[test]
    pub fn test_fn_without_args() {
        let mut l = Lexer::new("var variable = 6; fn test() {5;} test(); variable;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 6),
                JP(528),
                LDRegByte(0, 5),
                LDRegByte(14, 3),
                SubRegReg(13, 14),
                LDFReg(13),
                LDRegI(13),
                RET,
                LDFReg(13),
                LDIReg(13),
                LDRegByte(14, 3),
                AddRegReg(13, 14),
                CALL(516),
                LDRegReg(1, 0),
            ]
        ));
    }

    #[test]
    pub fn test_fn_nested_call_with_args() {
        let mut l =
            Lexer::new("var variable = 9; fn test(num) {var a = 5; num;} test(1); variable;");
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 9),
                JP(530),
                LDRegByte(1, 5),
                LDRegReg(2, 0),
                LDRegByte(14, 3),
                SubRegReg(13, 14),
                LDFReg(13),
                LDRegI(13),
                RET,
                LDFReg(13),
                LDIReg(13),
                LDRegByte(14, 3),
                AddRegReg(13, 14),
                LDRegByte(1, 1),
                LDRegReg(0, 1),
                CALL(516),
                LDRegReg(1, 0),
            ]
        ));
    }

    #[test]
    pub fn test_fn_with_args() {
        let mut l = Lexer::new(
            "var glob1 = 7;
            var glob2 = 3;
            
            fn doubleloop(num1, num2) {
              var num2backup = num2;
              while(num1 != 0) {
                 while(num2 != 0) {
                     num2 = num2 - 1;
                 }
               num2 = num2backup;
               num1 = num1 - 1;
              }
            }
            
            var glob3 = 255;
            
            doubleloop(glob2, glob1);
            
            var glob4 = 128;
            
            glob3;",
        );
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        c.compile();

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 7),
                LDRegByte(1, 3),
                JP(570),
                LDRegReg(2, 1),
                LDRegReg(3, 0),
                LDRegByte(4, 0),
                SNERegReg(3, 4),
                JP(560),
                LDRegReg(3, 1),
                LDRegByte(4, 0),
                SNERegReg(3, 4),
                JP(546),
                LDRegReg(3, 1),
                LDRegByte(4, 1),
                SubRegReg(3, 4),
                LDRegReg(1, 3),
                JP(528),
                LDRegReg(3, 2),
                LDRegReg(1, 3),
                LDRegReg(3, 0),
                LDRegByte(4, 1),
                SubRegReg(3, 4),
                LDRegReg(0, 3),
                JP(520),
                LDRegByte(14, 3),
                SubRegReg(13, 14),
                LDFReg(13),
                LDRegI(13),
                RET,
                LDRegByte(2, 255),
                LDFReg(13),
                LDIReg(13),
                LDRegByte(14, 3),
                AddRegReg(13, 14),
                LDRegReg(3, 1),
                LDRegReg(4, 0),
                LDRegReg(0, 3),
                LDRegReg(1, 4),
                CALL(518),
                LDRegByte(3, 128),
                LDRegReg(4, 2),
            ]
        ));
    }

    #[test]
    pub fn test_draw_rand_key_delay_I() {
        let mut l = Lexer::new(
            "
        var testvar = 10;

        fn drawrand(times, delay) {
            I = 20;
            while(times != 0) {
               times = times - 1;
               KEY();
               DT = delay;
               while (DT != 0) {}
               DRAW(RAND(255),RAND(255),5);
            }
        }   
        drawrand(testvar, 50);
        while(1 == 1) {7;}",
        );
        l.lex();
        let mut c = Compiler::new_from_lexer(&l);
        println!("TEST I");
        c.compile();

        for (pc, line) in &c.ram_line_map {
            println!("{}: {}", pc, line);
        }

        assert!(utils::vectors_equivalent(
            c.asm,
            vec![
                LDRegByte(0, 10),
                JP(568),
                LDIAddr(20),
                LDRegReg(2, 0),
                LDRegByte(3, 0),
                SNERegReg(2, 3),
                JP(558),
                LDRegReg(2, 0),
                LDRegByte(3, 1),
                SubRegReg(2, 3),
                LDRegReg(0, 2),
                LDRegKey(2),
                LDRegReg(2, 1),
                LDDTReg(2),
                LDRegDT(2),
                LDRegByte(3, 0),
                SNERegReg(2, 3),
                JP(550),
                JP(540),
                RNDRegByte(2, 255),
                RNDRegByte(3, 255),
                DRWRegRegNibble(2, 3, 5),
                JP(518),
                LDRegByte(14, 3),
                SubRegReg(13, 14),
                LDFReg(13),
                LDRegI(13),
                RET,
                LDFReg(13),
                LDIReg(13),
                LDRegByte(14, 3),
                AddRegReg(13, 14),
                LDRegReg(1, 0),
                LDRegByte(2, 50),
                LDRegReg(0, 1),
                LDRegReg(1, 2),
                CALL(516),
                LDRegByte(1, 1),
                LDRegByte(2, 1),
                SERegReg(1, 2),
                JP(598),
                LDRegByte(1, 7),
                //JP(588),
                JP(586),
            ]
        ));
    }
}
