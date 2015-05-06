extern crate byteorder;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

macro_rules! define_instructions {
    (variant, value, name, operands, stack_args, stack_effect
     $($variant:ident,
       $value:expr,
       $name:expr,
       $num_operands:expr,
       $num_stack_args:expr,
       $stack_effect:expr)*) => (

        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        #[repr(u8)]
        enum Inst { $($variant = $value),* }

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

            fn num_operands(self)   -> u8 { match self { $(Inst::$variant => $num_operands),* } }
            fn num_stack_args(self) -> u8 { match self { $(Inst::$variant => $num_stack_args),* } }
            fn stack_effect(self)   -> i8 { match self { $(Inst::$variant => $stack_effect),* } }
        }
    )
}

// Bytecode instruction opcodes. The values of these opcodes should never change, to remain
// compatible with existing bytecode programs.
define_instructions! {
    variant, value, name,    operands, stack_args, stack_effect
    Nop,     0,     "nop",   0,        0,           0
    Print,   1,     "print", 0,        1,          -1
    Halt,    2,     "halt",  0,        0,           0
    Push,    3,     "push",  1,        0,           1
    Dup,     4,     "dup",   0,        1,           1
    Pop,     5,     "pop",   0,        1,          -1
    Swap,    6,     "swap",  0,        2,           0
    Add,     7,     "add",   0,        2,          -1
    Sub,     8,     "sub",   0,        2,          -1
    Mul,     9,     "mul",   0,        2,          -1
    Div,     10,    "div",   0,        2,          -1
    Mod,     11,    "mod",   0,        2,          -1
    Eq,      12,    "eq",    0,        2,          -1
    Lt,      13,    "lt",    0,        2,          -1
    Lte,     14,    "lte",   0,        2,          -1
    Gt,      15,    "gt",    0,        2,          -1
    Gte,     16,    "gte",   0,        2,          -1
    Jz,      17,    "jz",    1,        1,          -1
    Jnz,     18,    "jnz",   1,        1,          -1
    Jump,    19,    "jump",  1,        0,           0
    Call,    20,    "call",  1,        0,           0
    Ret,     21,    "ret",   0,        0,           0
    CPush,   22,    "cpush", 0,        1,          -1
    CPop,    23,    "cpop",  0,        0,           1
    CDup,    24,    "cdup",  0,        0,           1
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
        #[inline(always)]
        fn jump(opcodes: &mut Cursor<&[u8]>, condition: bool) -> Result<(), VmError> {
            let delta = try!(opcodes.read_i32::<LittleEndian>().or(Err(UnexpectedProgramEnd)));
            if condition {
                let operand_size = std::mem::size_of::<i32>() as i64;
                let addr = (opcodes.position() as i64 + delta as i64 - operand_size) as u64;
                opcodes.set_position(addr);
            }
            Ok(())
        }

        use VmError::*;

        let mut opcodes = Cursor::new(program);

        while let Ok(opcode) = opcodes.read_u8() {
            let inst = try!(Inst::from_u8(opcode).ok_or(InvalidOpcode));

            if self.stack_idx < inst.num_stack_args() as usize { return Err(StackUnderflow); }
            if self.stack_idx as isize >= self.stack.len() as isize - inst.stack_effect() as isize {
                return Err(StackOverflow);
            }

            match inst {
                Inst::Nop => {}

                Inst::Print => {
                    let val = unsafe { *self.stack.get_unchecked(self.stack_idx - 1) };
                    println!("{}", val);
                }

                Inst::Halt => {
                    break;
                }

                Inst::Push => {
                    let val = try!(opcodes.read_i32::<LittleEndian>()
                                   .or(Err(UnexpectedProgramEnd)));
                    let stack_top = try!(self.stack.get_mut(self.stack_idx).ok_or(StackOverflow));
                    *stack_top = val;
                }

                Inst::Dup => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx) =
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Pop => {}

                Inst::Swap => {
                    unsafe {
                        let tmp = *self.stack.get_unchecked(self.stack_idx - 1);
                        *self.stack.get_unchecked_mut(self.stack_idx - 1) =
                            *self.stack.get_unchecked(self.stack_idx - 2);
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) = tmp;
                    }
                }

                Inst::Add => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) +=
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Sub => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) -=
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Mul => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) *=
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Div => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) /=
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Mod => {
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx - 2) %=
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                }

                Inst::Eq => {
                    unsafe {
                        let val1 = *self.stack.get_unchecked_mut(self.stack_idx - 1);
                        let ptr2 = self.stack.get_unchecked_mut(self.stack_idx - 2);
                        *ptr2 = (*ptr2 == val1) as i32;
                    }
                }

                Inst::Lt => {
                    unsafe {
                        let val1 = *self.stack.get_unchecked_mut(self.stack_idx - 1);
                        let ptr2 = self.stack.get_unchecked_mut(self.stack_idx - 2);
                        *ptr2 = (*ptr2 < val1) as i32;
                    }
                }

                Inst::Lte => {
                    unsafe {
                        let val1 = *self.stack.get_unchecked_mut(self.stack_idx - 1);
                        let ptr2 = self.stack.get_unchecked_mut(self.stack_idx - 2);
                        *ptr2 = (*ptr2 <= val1) as i32;
                    }
                }

                Inst::Gt => {
                    unsafe {
                        let val1 = *self.stack.get_unchecked_mut(self.stack_idx - 1);
                        let ptr2 = self.stack.get_unchecked_mut(self.stack_idx - 2);
                        *ptr2 = (*ptr2 > val1) as i32;
                    }
                }

                Inst::Gte => {
                    unsafe {
                        let val1 = *self.stack.get_unchecked_mut(self.stack_idx - 1);
                        let ptr2 = self.stack.get_unchecked_mut(self.stack_idx - 2);
                        *ptr2 = (*ptr2 >= val1) as i32;
                    }
                }

                Inst::Jz => {
                    let condition = unsafe { *self.stack.get_unchecked(self.stack_idx - 1) };
                    try!(jump(&mut opcodes, condition == 0));
                }

                Inst::Jnz => {
                    let condition = unsafe { *self.stack.get_unchecked(self.stack_idx - 1) };
                    try!(jump(&mut opcodes, condition != 0));
                }

                Inst::Jump => {
                    try!(jump(&mut opcodes, true));
                }

                Inst::Call => {
                    let control_stack_top = try!(self.control_stack.get_mut(self.control_stack_idx)
                                                 .ok_or(ControlStackOverflow));
                    *control_stack_top = opcodes.position() as i32 + 4;
                    try!(jump(&mut opcodes, true));
                    self.control_stack_idx += 1;
                }

                Inst::Ret => {
                    let addr = *try!(self.control_stack.get_mut(self.control_stack_idx - 1)
                                     .ok_or(ControlStackOverflow));
                    opcodes.set_position(addr as u64);
                    self.control_stack_idx -= 1;
                }

                Inst::CPush => {
                    if self.control_stack_idx >= self.control_stack.len() {
                        return Err(ControlStackOverflow);
                    }
                    unsafe {
                        *self.control_stack.get_unchecked_mut(self.control_stack_idx) =
                            *self.stack.get_unchecked(self.stack_idx - 1);
                    }
                    self.control_stack_idx += 1;
                }

                Inst::CPop => {
                    if self.control_stack_idx < 1 { return Err(ControlStackUnderflow); }
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx) =
                            *self.control_stack.get_unchecked(self.control_stack_idx - 1);
                    }
                    self.control_stack_idx -= 1;
                }

                Inst::CDup => {
                    if self.control_stack_idx < 1 { return Err(ControlStackUnderflow); }
                    unsafe {
                        *self.stack.get_unchecked_mut(self.stack_idx) =
                            *self.control_stack.get_unchecked(self.control_stack_idx - 1);
                    }
                }
            }

            self.stack_idx = (self.stack_idx as isize + inst.stack_effect() as isize) as usize;
        }

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
    // let source = r"
    //     push 1
// loop:   call @plus2
    //     dup; print
    //     jump @loop

// plus2:  push 2
    //     add
    //     ret
    // ";

    let source = r"
        push 10
        call @fact
        print
        halt

fact:   push 1
        swap
loop:   dup; jz @done
        dup; cpush
        mul
        cpop; push 1; sub
        jump @loop
done:   pop
        ret
    ";

    let program = assemble(source);

    let mut vm = Vm {
        stack: Box::new([0; 256]),
        stack_idx: 0,
        control_stack: Box::new([0; 256]),
        control_stack_idx: 0,
    };
    vm.execute(&program).unwrap();
}
