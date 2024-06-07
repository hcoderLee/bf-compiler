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
    // Clear current value, pattern: [Add(n)], n is odd
    Clear,
    // Add current value to the value in relative pos n, save the result to n, and clear current value
    // pattern: [Add(-1) Move(n) Add(1) Move(-n)]
    AddTo(isize),
    // Move until 0, pattern: [Move(n)]
    MoveUntil(isize),
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
                    let ix_len = instructions.len();
                    match bracket_stack.pop() {
                        Some(start) => Self::parse_loop(&mut instructions, start),
                        None => return Err(UnbalancedBrackets(']', ix_len)),
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

    /// Optimize loop
    fn parse_loop(instructions: &mut Vec<Instruction>, start: usize) -> Instruction {
        let ix_len = instructions.len();
        instructions[start] = Instruction::LoopStart(ix_len);
        match instructions.as_slice() {
            // Parse Clear
            &[.., Instruction::LoopStart(_), Instruction::Add(n)] if n % 2 == 1 => {
                instructions.drain(ix_len - 2..);
                Instruction::Clear
            }
            // Parse AddTo
            &[.., Instruction::LoopStart(_), Instruction::Add(255), Instruction::Move(n), Instruction::Add(1), Instruction::Move(m)]
                if n == -m =>
            {
                instructions.drain(ix_len - 5..);
                Instruction::AddTo(n)
            }
            // parse MoveUntil
            &[.., Instruction::LoopStart(_), Instruction::Move(n)] => {
                instructions.drain(ix_len - 2..);
                Instruction::MoveUntil(n)
            }
            _ => Instruction::LoopEnd(start),
        }
    }

    pub fn run(&mut self) -> std::io::Result<()> {
        let mut stdout = stdout().lock();
        let mut stdin = std::io::stdin().lock();
        'program: loop {
            let instruction = self.instructions[self.inst_ptr];
            match instruction {
                Instruction::Move(n) => {
                    self.mem_ptr = self.get_pos(n);
                }
                Instruction::Add(n) => {
                    self.set_cur_value(self.cur_value().wrapping_add(n));
                }
                Instruction::OutputValue => {
                    stdout.write_all(&[self.cur_value()])?;
                    stdout.flush()?;
                }
                Instruction::InputValue => loop {
                    let read_res =
                        stdin.read_exact(&mut self.memory[self.mem_ptr..self.mem_ptr + 1]);
                    match read_res.as_ref().map_err(|e| e.kind()) {
                        Err(std::io::ErrorKind::UnexpectedEof) => {
                            self.set_cur_value(0);
                        }
                        _ => read_res?,
                    }
                    if self.cur_value() == b'\r' {
                        continue;
                    }
                    break;
                },
                Instruction::LoopStart(end) => {
                    if self.cur_value() == 0 {
                        self.inst_ptr = end;
                    }
                }
                Instruction::LoopEnd(start) => {
                    if self.cur_value() != 0 {
                        self.inst_ptr = start;
                    }
                }
                Instruction::Clear => self.set_cur_value(0),
                Instruction::AddTo(n) => {
                    let to = self.get_pos(n);
                    self.memory[to] = self.cur_value().wrapping_add(self.memory[to]);
                    self.set_cur_value(0);
                }
                Instruction::MoveUntil(n) => loop {
                    if self.cur_value() == 0 {
                        break;
                    }
                    self.mem_ptr = self.get_pos(n);
                },
            }
            self.inst_ptr += 1;
            if self.inst_ptr == self.instructions.len() {
                break 'program;
            }
        }

        Ok(())
    }

    #[inline]
    fn cur_value(&self) -> u8 {
        self.memory[self.mem_ptr]
    }

    #[inline]
    fn set_cur_value(&mut self, value: u8) {
        self.memory[self.mem_ptr] = value;
    }

    /// Get the `offset` position related to the current memory pointer
    #[inline]
    fn get_pos(&self, offset: isize) -> usize {
        (self.mem_ptr as isize + offset + MAX_MEM_ARRAY_SIZE as isize) as usize % MAX_MEM_ARRAY_SIZE
    }
}
