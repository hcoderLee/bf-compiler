use std::io::{stdout, Read, Write};

const MAX_MEM_ARRAY_SIZE: usize = 30000;

#[derive(PartialEq, Eq, Copy, Clone, Debug)]
enum Instruction {
    IncPtr,      // >
    DecPtr,      // <
    IncValue,    // +
    DecValue,    // -
    OutputValue, // .
    InputValue,  // ,
    LoopStart,   // [
    LoopEnd,     // ]
}

pub struct Program {
    instructions: Vec<Instruction>,
    memory: [u8; MAX_MEM_ARRAY_SIZE],
    mem_ptr: usize,
    inst_ptr: usize,
}

impl Program {
    pub fn new(code: &[u8]) -> Program {
        let mut instructions = Vec::new();
        for &c in code {
            let instruction = match c {
                b'>' => Instruction::IncPtr,
                b'<' => Instruction::DecPtr,
                b'+' => Instruction::IncValue,
                b'-' => Instruction::DecValue,
                b'.' => Instruction::OutputValue,
                b',' => Instruction::InputValue,
                b'[' => Instruction::LoopStart,
                b']' => Instruction::LoopEnd,
                _ => continue,
            };
            instructions.push(instruction);
        }
        Program {
            instructions,
            memory: [0; MAX_MEM_ARRAY_SIZE],
            mem_ptr: 0,
            inst_ptr: 0,
        }
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout().lock();
        let mut stdin = std::io::stdin().lock();
        'program: loop {
            let instruction = self.instructions[self.inst_ptr];
            match instruction {
                Instruction::IncPtr => {
                    self.mem_ptr = (self.mem_ptr + 1) % MAX_MEM_ARRAY_SIZE;
                }
                Instruction::DecPtr => {
                    self.mem_ptr = (self.mem_ptr + MAX_MEM_ARRAY_SIZE - 1) % MAX_MEM_ARRAY_SIZE;
                }
                Instruction::IncValue => {
                    self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_add(1);
                }
                Instruction::DecValue => {
                    self.memory[self.mem_ptr] = self.memory[self.mem_ptr].wrapping_sub(1);
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
                Instruction::LoopStart => {
                    if self.memory[self.mem_ptr] == 0 {
                        let mut depth = 1;
                        loop {
                            if self.inst_ptr == self.instructions.len() - 1 {
                                break 'program;
                            }
                            self.inst_ptr += 1;
                            match self.instructions[self.inst_ptr] {
                                Instruction::LoopStart => depth += 1,
                                Instruction::LoopEnd => depth -= 1,
                                _ => {}
                            }
                            if depth == 0 {
                                break;
                            }
                        }
                    }
                }
                Instruction::LoopEnd => {
                    if self.memory[self.mem_ptr] != 0 {
                        let mut depth = 1;
                        loop {
                            if self.inst_ptr == 0 {
                                break 'program;
                            }
                            self.inst_ptr -= 1;
                            match self.instructions[self.inst_ptr] {
                                Instruction::LoopStart => depth -= 1,
                                Instruction::LoopEnd => depth += 1,
                                _ => {}
                            }
                            if depth == 0 {
                                break;
                            }
                        }
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
