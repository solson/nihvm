extern crate byteorder;
use std::io::Cursor;
use byteorder::{LittleEndian, ReadBytesExt};

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
            // This is safe as long as MAX_INST_VARIANT
            Some(unsafe { std::mem::transmute::<u8, Self>(x) })
        } else {
            None
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
                let delta = try!(opcodes.read_i8().or(Err(UnexpectedProgramEnd)));
                let addr = (opcodes.position() as i64 + delta as i64) as u64;
                opcodes.set_position(addr);
            },
        }

        //println!("{:?}", &stack[..stack_idx]);
    }

    Ok(stack_idx)
}

fn main() {
    let program = &[
        Inst::Push as u8, 1, 0, 0, 0,
        Inst::Push as u8, 2, 0, 0, 0,
        Inst::Add as u8,
        Inst::Dup as u8,
        Inst::Print as u8,
        Inst::Halt as u8,
        Inst::Jump as u8, -10i8 as u8,
    ];
    execute(program, &mut [0; 256], 0).unwrap();
}
