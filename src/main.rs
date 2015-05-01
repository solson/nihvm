extern crate byteorder;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

macro_rules! define_instructions {
    ($($variant:ident, $value:expr, $name:expr, $num_operands:expr;)*) => (
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[repr(u8)]
        enum Inst {
            $($variant = $value),*
        }

        impl Inst {
            fn from_u8(inst: u8) -> Option<Inst> {
                match inst {
                    $($value => Some(Inst::$variant),)*
                    _ => None
                }
            }

            fn from_str(inst: &str) -> Option<Inst> {
                match inst {
                    $($name => Some(Inst::$variant),)*
                    _ => None
                }
            }

            fn num_operands(&self) -> u8 {
                match *self {
                    $(Inst::$variant => $num_operands),*
                }
            }
        }
    )
}

// Bytecode instruction opcodes. The values of these opcodes should never change, to remain
// compatible with existing bytecode programs.
define_instructions! {
    Nop,   0,  "nop",   0;
    Print, 1,  "print", 0;
    Halt,  2,  "halt",  0;
    Push,  3,  "push",  1;
    Dup,   4,  "dup",   0;
    Pop,   5,  "pop",   0;
    Swap,  6,  "swap",  0;
    Add,   7,  "add",   0;
    Sub,   8,  "sub",   0;
    Mul,   9,  "mul",   0;
    Div,   10, "div",   0;
    Mod,   11, "mod",   0;
    Eq,    12, "eq",    0;
    Lt,    13, "lt",    0;
    Lte,   14, "lte",   0;
    Gt,    15, "gt",    0;
    Gte,   16, "gte",   0;
    Jz,    17, "jz",    1;
    Jnz,   18, "jnz",   1;
    Jump,  19, "jump",  1;
    Call,  20, "call",  1;
    Ret,   21, "ret",   0;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VmError {
    InvalidOpcode,
    UnexpectedProgramEnd, // Hit end of program while reading operand.
    StackOverflow,
    StackUnderflow,
    ControlStackOverflow,
    ControlStackUnderflow,
}

#[derive(Debug, Eq, PartialEq)]
struct Vm {
    stack: Box<[i32]>,
    stack_idx: usize,
    control_stack: Box<[i32]>,
    control_stack_idx: usize,
}

impl Vm {
    fn execute(&mut self, program: &[u8]) -> Result<(), VmError> {
        use VmError::*;

        let stack = &mut self.stack;
        let mut stack_idx = self.stack_idx;
        let mut opcodes = Cursor::new(program);

        while let Ok(opcode) = opcodes.read_u8() {
            let inst = try!(Inst::from_u8(opcode).ok_or(InvalidOpcode));
            match inst {
                Inst::Nop => {},

                Inst::Print => {
                    let val = *try!(stack.get(stack_idx - 1).ok_or(StackUnderflow));
                    println!("{}", val);
                    stack_idx -= 1;
                },

                Inst::Halt => {
                    break;
                },

                Inst::Push => {
                    let val = try!(opcodes.read_i32::<LittleEndian>().or(Err(UnexpectedProgramEnd)));
                    let stack_top = try!(stack.get_mut(stack_idx).ok_or(StackOverflow));
                    *stack_top = val;
                    stack_idx += 1;
                },

                Inst::Dup => {
                    if stack_idx < 1 { return Err(StackUnderflow); }
                    if stack_idx >= stack.len() { return Err(StackOverflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx) = *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx += 1;
                },

                Inst::Pop => {
                    if stack_idx < 1 { return Err(StackUnderflow); }
                    stack_idx -= 1;
                },

                Inst::Swap => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let tmp = *stack.get_unchecked(stack_idx - 1);
                        *stack.get_unchecked_mut(stack_idx - 1) = *stack.get_unchecked(stack_idx - 2);
                        *stack.get_unchecked_mut(stack_idx - 2) = tmp;
                    }
                },

                Inst::Add => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx - 2) += *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx -= 1;
                },

                Inst::Sub => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx - 2) -= *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx -= 1;
                },

                Inst::Mul => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx - 2) *= *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx -= 1;
                },

                Inst::Div => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx - 2) /= *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx -= 1;
                },

                Inst::Mod => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        *stack.get_unchecked_mut(stack_idx - 2) %= *stack.get_unchecked(stack_idx - 1);
                    }
                    stack_idx -= 1;
                },

                Inst::Eq => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let val1 = *stack.get_unchecked_mut(stack_idx - 1);
                        let ptr2 = stack.get_unchecked_mut(stack_idx - 2);
                        *ptr2 = (*ptr2 == val1) as i32;
                    }
                    stack_idx -= 1;
                },

                Inst::Lt => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let val1 = *stack.get_unchecked_mut(stack_idx - 1);
                        let ptr2 = stack.get_unchecked_mut(stack_idx - 2);
                        *ptr2 = (*ptr2 < val1) as i32;
                    }
                    stack_idx -= 1;
                },

                Inst::Lte => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let val1 = *stack.get_unchecked_mut(stack_idx - 1);
                        let ptr2 = stack.get_unchecked_mut(stack_idx - 2);
                        *ptr2 = (*ptr2 <= val1) as i32;
                    }
                    stack_idx -= 1;
                },

                Inst::Gt => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let val1 = *stack.get_unchecked_mut(stack_idx - 1);
                        let ptr2 = stack.get_unchecked_mut(stack_idx - 2);
                        *ptr2 = (*ptr2 > val1) as i32;
                    }
                    stack_idx -= 1;
                },

                Inst::Gte => {
                    if stack_idx < 2 { return Err(StackUnderflow); }
                    unsafe {
                        let val1 = *stack.get_unchecked_mut(stack_idx - 1);
                        let ptr2 = stack.get_unchecked_mut(stack_idx - 2);
                        *ptr2 = (*ptr2 >= val1) as i32;
                    }
                    stack_idx -= 1;
                },

                Inst::Jz => {
                    if stack_idx < 1 { return Err(StackUnderflow); }
                    let condition = unsafe { *stack.get_unchecked(stack_idx - 1) };
                    if condition == 0 { try!(jump(&mut opcodes)); }
                    stack_idx -= 1;
                },

                Inst::Jnz => {
                    if stack_idx < 1 { return Err(StackUnderflow); }
                    let condition = unsafe { *stack.get_unchecked(stack_idx - 1) };
                    if condition != 0 { try!(jump(&mut opcodes)); }
                    stack_idx -= 1;
                },

                Inst::Jump => {
                    try!(jump(&mut opcodes));
                },

                Inst::Call => {
                    let control_stack_top = try!(self.control_stack.get_mut(self.control_stack_idx).ok_or(ControlStackOverflow));
                    *control_stack_top = opcodes.position() as i32 + 4;
                    try!(jump(&mut opcodes));
                    self.control_stack_idx += 1;
                },

                Inst::Ret => {
                    let addr = *try!(self.control_stack.get_mut(self.control_stack_idx - 1).ok_or(ControlStackOverflow));
                    opcodes.set_position(addr as u64);
                    self.control_stack_idx -= 1;
                },
            }
        }

        fn jump(opcodes: &mut Cursor<&[u8]>) -> Result<(), VmError> {
            let delta = try!(opcodes.read_i32::<LittleEndian>().or(Err(UnexpectedProgramEnd)));
            let operand_size = std::mem::size_of::<i32>() as i64;
            let addr = (opcodes.position() as i64 + delta as i64 - operand_size) as u64;
            opcodes.set_position(addr);
            Ok(())
        }

        self.stack_idx = stack_idx;
        Ok(())
    }
}

fn assemble(source: &str) -> Vec<u8> {
    use std::collections::HashMap;

    let mut program: Vec<u8> = Vec::new();
    let mut label_definitions: HashMap<&str, usize> = HashMap::new();
    let mut label_uses: Vec<(&str, usize)> = Vec::new();

    for line in source.split(|c| c == '\n' || c == ';') {
        let mut tokens = line.split(char::is_whitespace).filter(|s| !s.is_empty());
        let mut first_token = tokens.next();

        // Parse an optional label at the start of the line.
        if let Some(label) = first_token {
            if label.chars().next_back() == Some(':') {
                let label_name = &label[..label.len() - 1];
                if label_definitions.insert(label_name, program.len()).is_some() {
                    panic!("Attempted to redefine label '{}'", label_name);
                }
                first_token = tokens.next()
            }
        }

        // Parse the rest of the line if it's not blank.
        if let Some(opcode) = first_token {
            // Parse the instruction name.
            let inst = Inst::from_str(opcode).unwrap_or_else(|| {
                panic!("Unrecognized instruction '{}'", opcode)
            });
            program.push(inst as u8);

            // Parse the operands.
            for _ in 0..inst.num_operands() {
                let operand = tokens.next().unwrap_or_else(|| {
                    panic!("Missing one or more operands after '{}'", opcode)
                });

                if operand.chars().next() == Some('@') {
                    let label_name = &operand[1..];
                    label_uses.push((label_name, program.len()));

                    // Push four zero bytes to be overwritten by the label location later.
                    for _ in 0..4 { program.push(0); }
                } else if let Ok(number) = operand.parse::<i32>() {
                    let operand_index = program.len();
                    for _ in 0..4 { program.push(0); }
                    (&mut program[operand_index..]).write_i32::<LittleEndian>(number).unwrap();
                } else {
                    panic!("Expected label or valid 32-bit signed integer after '{}', not '{}'",
                           opcode, operand);
                }
            }
        }
    }

    // Resolve label references and fill in their values in the bytecode.
    for (label_name, use_index) in label_uses {
        let target_index = *label_definitions.get(label_name).unwrap_or_else(|| {
            panic!("Reference to undefined label '{}'", label_name);
        });
        let delta = target_index as i32 - use_index as i32;
        (&mut program[use_index..]).write_i32::<LittleEndian>(delta).unwrap();
    }

    program
}

fn main() {
    let source = r"
        push 1
loop:   call @plus2
        dup; print
        jump @loop

plus2:  push 2
        add
        ret
    ";

    // let source = r"
    //     push 1
    //     push 10
// loop:   dup; push 0; eq; jnz @done
    //     jump @loop
// done:   print
    // ";

    let program = assemble(source);

    let mut vm = Vm {
        stack: Box::new([0; 256]),
        stack_idx: 0,
        control_stack: Box::new([0; 256]),
        control_stack_idx: 0,
    };
    vm.execute(&program).unwrap();
}
