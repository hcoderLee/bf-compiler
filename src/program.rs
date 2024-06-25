use crate::program::Instruction::*;
use std::io::Write;

const MAX_MEM_ARRAY_SIZE: usize = 30000;

pub struct UnbalancedBrackets(pub char, pub usize);

pub struct Program {
    code: Vec<u8>,
    memory: [u8; MAX_MEM_ARRAY_SIZE],
}

#[derive(Debug)]
enum Instruction {
    Increment(i64),
    Move(isize),
    Input,
    Output,
    LoopStart(usize),
    LoopEnd(usize),
    // Clear current value, pattern: [Add(n)], n is odd
    Clear,
    // Add current value to the value in relative pos n, save the result to n, and clear current value
    // pattern: [Add(-1) Move(n) Add(1) Move(-n)]
    AddTo(isize),
}

impl Instruction {
    fn code_len(&self) -> usize {
        match self {
            Increment(_) => 16,
            Move(_) => 16,
            Input => 20,
            Output => 20,
            LoopStart(_) => 12,
            LoopEnd(_) => 12,
            Clear => 4,
            AddTo(_) => 36,
        }
    }
}

impl Program {
    pub fn new(source: &[u8]) -> Result<Program, UnbalancedBrackets> {
        let instructions = Program::parse(source)?;
        let code = Program::compile(instructions);
        let memory = [0u8; MAX_MEM_ARRAY_SIZE];
        Ok(Program { code, memory })
    }

    fn parse(source: &[u8]) -> Result<Vec<Instruction>, UnbalancedBrackets> {
        // The stack store [ instruction, the value is : (char index, instruction index, code index)
        let mut bracket_stack: Vec<(usize, usize, usize)> = Vec::new();
        let mut instructions: Vec<Instruction> = Vec::new();

        let mut code_len = 16;
        for (i, c) in source.iter().enumerate() {
            match c {
                b'+' | b'-' => {
                    let inc = if *c == b'+' { 1 } else { -1 };
                    if let Some(Increment(v)) = instructions.last_mut() {
                        *v += inc;
                        if *v == 0 {
                            instructions.pop();
                        }
                        continue;
                    }
                    let ix = Increment(inc);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'>' | b'<' => {
                    let step = if *c == b'>' { 1 } else { -1 };
                    if let Some(Move(v)) = instructions.last_mut() {
                        *v += step;
                        if *v == 0 {
                            instructions.pop();
                        }
                        continue;
                    }
                    let ix = Move(step);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'.' => {
                    let ix = Output;
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b',' => {
                    let ix = Input;
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'[' => {
                    bracket_stack.push((i, instructions.len(), code_len));
                    let ix = LoopStart(0);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b']' => {
                    let b_start = bracket_stack.pop();
                    if b_start.is_none() {
                        return Err(UnbalancedBrackets(']', i));
                    }

                    let (_, ix_i, code_i) = b_start.unwrap();
                    let ix_len = instructions.len();
                    instructions[ix_i] = LoopStart(code_len);
                    let ix = match instructions.as_slice() {
                        // Parse Clear
                        &[.., LoopStart(_), Increment(n)] if n % 2 == 1 => {
                            instructions
                                .drain(ix_len - 2..)
                                .for_each(|i| code_len -= i.code_len());
                            Clear
                        }
                        // Parse AddTo
                        &[.., LoopStart(_), Increment(255), Move(n), Increment(1), Move(m)]
                            if n == -m =>
                        {
                            instructions
                                .drain(ix_len - 5..)
                                .for_each(|i| code_len -= i.code_len());
                            AddTo(n)
                        }
                        _ => LoopEnd(code_i),
                    };

                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                _ => continue,
            }
        }

        if !bracket_stack.is_empty() {
            return Err(UnbalancedBrackets('[', bracket_stack.pop().unwrap().0));
        }

        return Ok(instructions);
    }

    fn compile(instructions: Vec<Instruction>) -> Vec<u8> {
        // Prolouge
        let mut code: Vec<u8> = vec![
            0xff, 0x43, 0x00, 0xd1, // sub	sp, sp, #16
            0xfd, 0x7b, 0x00, 0xa9, // stp	x29, x30, [sp]
            0xe8, 0x03, 0x00, 0xaa, // mov x8, x0 ; x8: self.memory
            0x09, 0x00, 0x80, 0xd2, // mov x9, #0 ; x9: current memory pointer
        ];

        for ix in instructions {
            match ix {
                Increment(inc) => {
                    let inc = inc as u32 & 0x0fff;
                    let add = (0x110 << 20 | inc << 10 | 0x14a).to_le_bytes();
                    code.write_all(&[
                        0x0b, 0x01, 0x09, 0x8b, // add	x11, x8, x9
                        0x6a, 0x01, 0x40, 0x39, // ldrb	w10, [x11]
                        add[0], add[1], add[2], add[3], // add	w10, w10, #count
                        0x6a, 0x01, 0x00, 0x39, // strb	w10, [x11]
                    ])
                    .unwrap();
                }
                Move(step) => code.write_all(&Self::move_instr_codes(step, 0xa)).unwrap(),
                Input => {
                    code.write_all(&[
                        0x00, 0x00, 0x80, 0x52, // mov w0,  #1 ; fd: stdin
                        0x01, 0x01, 0x09, 0x8b, // add x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov x2,  #1 ; input len, only 1 number
                        0x70, 0x00, 0x80, 0x52, // mov w16, #3 ; read system call number
                        0x01, 0x10, 0x00, 0xd4, // svc #0x80
                    ])
                    .unwrap()
                }
                Output => {
                    code.write_all(&[
                        0x20, 0x00, 0x80, 0x52, // mov	w0,  #0 ; fd: stdout
                        0x01, 0x01, 0x09, 0x8b, // add  x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov	x2,  #1 ; output len, only 1 number
                        0x90, 0x00, 0x80, 0x52, // mov	w16, #4 ; write system call number
                        0x01, 0x10, 0x00, 0xd4, // svc	#0x80    ;  syscall
                    ])
                    .unwrap()
                }
                LoopStart(end) => {
                    let start = code.len();
                    let offset = (end - start) as isize + 4;
                    let b_eq = aarch64_b_eq_ix(offset).to_le_bytes();
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	 w10, [x8, x9]
                        0x5f, 0x01, 0x0a, 0x6a, // tst	 w10, xzr
                        b_eq[0], b_eq[1], b_eq[2], b_eq[3], // b.eq	 <end>
                    ])
                    .unwrap();
                }
                LoopEnd(start) => {
                    let end = code.len();
                    let offset = ((((start - end + 4) as i32) >> 2) as u32) & 0x7ffff;
                    let b_ne = (0x54 << 24 | offset << 5 | 0x1).to_le_bytes();
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	 w10, [x8, x9]
                        0x5f, 0x01, 0x0a, 0x6a, // tst	 w10, w10
                        b_ne[0], b_ne[1], b_ne[2], b_ne[3], // b.ne	 <start>
                    ])
                    .unwrap();
                }
                Clear => code
                    .write_all(&[
                        0x1f, 0x69, 0x29, 0x38, // strb	wzr, [x8, x9]
                    ])
                    .unwrap(),
                AddTo(n) => {
                    code.write_all(&Self::move_instr_codes(n, 0xb)).unwrap();
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	w10, [x8, x9]
                        0x0c, 0x69, 0x6b, 0x38, // ldrb	w12, [x8, x11]
                        0x4a, 0x01, 0x0c, 0x0b, // add	w10, w10, w12
                        0x0a, 0x69, 0x2b, 0x38, // strb	w10, [x8, x11]
                        0x1f, 0x69, 0x29, 0x38, //strb	wzr, [x8, x9]
                    ])
                    .unwrap();
                }
            }
        }

        // Epilouge
        code.write_all(&[
            // 0x00, 0x69, 0x69, 0x38, //  test current value: ldrb	w0, [x8, x9]
            // 0xe0, 0x03, 0x09, 0xaa, // test current position: mov	x0, x9
            0xfd, 0x7b, 0x40, 0xa9, // ldp	x29, x30, [sp]
            0xff, 0x43, 0x00, 0x91, // add	sp,  sp,  #16
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ])
        .unwrap();

        // code.chunks(4).for_each(|chunk| {
        //     println!(
        //         "{:08x} ",
        //         u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        //     )
        // });

        return code;
    }

    fn move_instr_codes(n: isize, rd: u8) -> [u8; 16] {
        if n > 0 {
            // > instruction
            let n = n as u32 & 0x0fff;
            let add = (0x910 << 20 | n << 10 | 0x120 | rd as u32).to_le_bytes();
            [
                add[0], add[1], add[2], add[3], // add	x10, x9, #step
                0x4b, 0xe1, 0x2e, 0xd1, // sub	x11, x10, #3000
                0x5f, 0xdd, 0x2e, 0xf1, //cmp	x10, #2999
                0x69, 0xc1, 0x8a, 0x9a, // csel	x9, x11, x10, gt
            ]
        } else {
            // < instruction
            let n = -n as u32 & 0x0fff;
            let sub = (0xd10 << 20 | n << 10 | 0x12a).to_le_bytes();
            [
                sub[0], sub[1], sub[2], sub[3], // add x10, x9, #step
                0x4b, 0xe1, 0x2e, 0x91, // add	x11, x10, #3000
                0x5f, 0x01, 0x00, 0xf1, // cmp	x10, #0
                0x69, 0xb1, 0x8a, 0x9a, // csel	x9, x11, x10, lt
            ]
        }
    }

    #[inline(never)]
    pub fn run(&mut self) -> std::io::Result<()> {
        unsafe {
            let code_mem = libc::mmap(
                std::ptr::null_mut(),
                self.code.len(),
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            );
            if code_mem == libc::MAP_FAILED {
                panic!("mmap failed. {:?}", get_os_err());
            }

            std::slice::from_raw_parts_mut(code_mem as *mut u8, self.code.len())
                .copy_from_slice(&self.code);

            let res = libc::mprotect(code_mem, self.code.len(), libc::PROT_READ | libc::PROT_EXEC);
            if res == -1 {
                panic!("mprotect failed. {}", get_os_err());
            }

            let _run: extern "Rust" fn(*mut u8) = std::mem::transmute(code_mem);

            _run(self.memory.as_mut_ptr());

            let res = libc::munmap(code_mem, self.code.len());
            if res == -1 {
                panic!("munmap failed. {}", get_os_err());
            }
        }

        Ok(())
    }
}

fn aarch64_b_eq_ix(offset: isize) -> u32 {
    let offset = (offset >> 2) as u32 & 0x7ffff;
    0x54 << 24 | offset << 5
}

fn get_os_err() -> std::io::Error {
    unsafe { std::io::Error::from_raw_os_error(*libc::__error()) }
}
