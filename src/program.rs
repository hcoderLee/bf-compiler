use std::io::{stdout, Read, Write};

const MAX_MEM_ARRAY_SIZE: usize = 30000;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Instruction {
    Move(isize),      // > or <
    Add(u8),          // + or -
    OutputValue,      // .
    InputValue,       // ,
    LoopStart(usize), // [
    LoopEnd(usize),   // ]
}

pub struct UnbalancedBrackets(pub char, pub usize);

pub struct Program {
    instructions: Vec<Instruction>,
    memory: [u8; MAX_MEM_ARRAY_SIZE],
    mem_ptr: usize,
    inst_ptr: usize,
}

impl Program {
    pub fn new(code: &[u8]) -> Result<Program, UnbalancedBrackets> {
        let mut instructions = Vec::new();
        let mut bracket_stack = Vec::new();
        for &c in code {
            let instruction = match c {
                b'>' | b'<' => {
                    let offset = if c == b'>' { 1 } else { -1 };
                    if let Some(Instruction::Move(last_offset)) = instructions.last_mut() {
                        *last_offset += offset;
                        continue;
                    }
                    Instruction::Move(offset)
                }
                b'+' | b'-' => {
                    let inc = if c == b'+' { 1 } else { 1u8.wrapping_neg() };
                    if let Some(Instruction::Add(last_inc)) = instructions.last_mut() {
                        *last_inc = last_inc.wrapping_add(inc);
                        continue;
                    }
                    Instruction::Add(inc)
                }
                b'.' => Instruction::OutputValue,
                b',' => Instruction::InputValue,
                b'[' => {
                    bracket_stack.push(instructions.len());
                    Instruction::LoopStart(0)
                }
                b']' => {
                    let cur_addr = instructions.len();
                    match bracket_stack.pop() {
                        Some(start) => {
                            instructions[start] = Instruction::LoopStart(cur_addr);
                            Instruction::LoopEnd(start)
                        }
                        None => return Err(UnbalancedBrackets(']', cur_addr)),
                    }
                }
                _ => continue,
            };
            instructions.push(instruction);
        }

        if let Some(start) = bracket_stack.pop() {
            return Err(UnbalancedBrackets('[', start));
        }

        Ok(Program {
            instructions,
            memory: [0; MAX_MEM_ARRAY_SIZE],
            mem_ptr: 0,
            inst_ptr: 0,
        })
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout().lock();
        let mut stdin = std::io::stdin().lock();
        'program: loop {
            let instruction = self.instructions[self.inst_ptr];
            match instruction {
                Instruction::Move(n) => {
                    let steps = (MAX_MEM_ARRAY_SIZE as isize + n) as usize % MAX_MEM_ARRAY_SIZE;
                    self.mem_ptr = (self.mem_ptr + steps) % MAX_MEM_ARRAY_SIZE;
                }
                Instruction::Add(n) => {
                    self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_add(n);
                }
                Instruction::OutputValue => {
                    let value = self.memory[self.mem_ptr];
                    stdout.write_all(&[value])?;
                    stdout.flush()?;
                }
                Instruction::InputValue => loop {
                    let read_res =
                        stdin.read_exact(&mut self.memory[self.mem_ptr..self.mem_ptr + 1]);
                    match read_res.as_ref().map_err(|e| e.kind()) {
                        Err(std::io::ErrorKind::UnexpectedEof) => {
                            self.memory[self.mem_ptr] = 0;
                        }
                        _ => read_res?,
                    }
                    if self.memory[self.mem_ptr] == b'\r' {
                        continue;
                    }
                    break;
                },
                Instruction::LoopStart(end) => {
                    if self.memory[self.mem_ptr] == 0 {
                        self.inst_ptr = end;
                    }
                }
                Instruction::LoopEnd(start) => {
                    if self.memory[self.mem_ptr] != 0 {
                        self.inst_ptr = start;
                    }
                }
            }
            self.inst_ptr += 1;
            if self.inst_ptr == self.instructions.len() {
                break 'program;
            }
        }

        Ok(())
    }
}
