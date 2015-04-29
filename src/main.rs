extern crate byteorder;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};

// Bytecode instruction opcodes. The values of these opcodes should never
// change, to remain compatible with existing bytecode programs.
//
// To make the from_u8 function work, MAX_INST_VARIANT must be kept in sync with
// this enum and every value from 0 to MAX_INST_VARIANT should be assigned to an
// opcode.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
enum Inst {
    Nop   = 0,
    Push  = 1,
    Dup   = 2,
    Pop   = 3,
    Swap  = 4,
    Add   = 5,
    Print = 6,
    Halt  = 7,
    Jump  = 8,
}
const MAX_INST_VARIANT: u8 = 8;

impl Inst {
    fn from_u8(x: u8) -> Option<Self> {
        if x <= MAX_INST_VARIANT {
            // This is safe as long as MAX_INST_VARIANT is correct and all the valid enum values
            // are contiguous.
            Some(unsafe { std::mem::transmute::<u8, Self>(x) })
        } else {
            None
        }
    }

    fn num_operands(&self) -> u8 {
        match *self {
            Inst::Push | Inst::Jump => 1,
            _ => 0,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VmError {
    InvalidOpcode,
    UnexpectedProgramEnd, // Hit end of program while reading opcode argument.
    StackOverflow,
    StackUnderflow,
}

fn execute(program: &[u8],
           stack: &mut [i32],
           mut stack_idx: usize) -> Result<usize, VmError> {
    use VmError::*;

    let stack_size = stack.len();
    let mut opcodes = Cursor::new(program);

    while let Ok(opcode) = opcodes.read_u8() {
        let inst = try!(Inst::from_u8(opcode).ok_or(InvalidOpcode));
        match inst {
            Inst::Nop => {},

            Inst::Push => {
                let val = try!(opcodes.read_i32::<LittleEndian>().or(Err(UnexpectedProgramEnd)));
                let stack_top = try!(stack.get_mut(stack_idx).ok_or(StackOverflow));
                *stack_top = val;
                stack_idx += 1;
            },

            Inst::Dup => {
                if stack_idx < 1 { return Err(StackUnderflow); }
                if stack_idx >= stack_size { return Err(StackOverflow); }
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

            Inst::Print => {
                let val = *try!(stack.get(stack_idx - 1).ok_or(StackUnderflow));
                println!("{}", val);
                stack_idx -= 1;
            },

            Inst::Halt => {
                break;
            },

            Inst::Jump => {
                let delta = try!(opcodes.read_i32::<LittleEndian>().or(Err(UnexpectedProgramEnd)));
                let operand_size = std::mem::size_of::<i32>() as i64;
                let addr = (opcodes.position() as i64 + delta as i64 - operand_size) as u64;
                opcodes.set_position(addr);
            },
        }

        //println!("{:?}", &stack[..stack_idx]);
    }

    Ok(stack_idx)
}

fn parse_instruction(inst: &str) -> Option<Inst> {
    match inst {
        "nop"   => Some(Inst::Nop),
        "push"  => Some(Inst::Push),
        "dup"   => Some(Inst::Dup),
        "pop"   => Some(Inst::Pop),
        "swap"  => Some(Inst::Swap),
        "add"   => Some(Inst::Add),
        "print" => Some(Inst::Print),
        "halt"  => Some(Inst::Halt),
        "jump"  => Some(Inst::Jump),
        _       => None
    }
}

fn assemble(source: &str) -> Vec<u8> {
    use std::collections::HashMap;

    let mut program: Vec<u8> = Vec::new();
    let mut label_definitions: HashMap<&str, usize> = HashMap::new();
    let mut label_uses: Vec<(&str, usize)> = Vec::new();

    for line in source.lines() {
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
            let inst = parse_instruction(opcode).unwrap_or_else(|| {
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
        label: push 2
        add
        dup
        print
        jump @label
    ";

    let program = assemble(source);

    execute(&program, &mut [0; 256], 0).unwrap();
}
