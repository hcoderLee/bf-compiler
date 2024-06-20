use std::io::Write;

const MAX_MEM_ARRAY_SIZE: usize = 30000;

pub struct UnbalancedBrackets(pub char, pub usize);

pub struct Program {
    code: Vec<u8>,
    memory: [u8; MAX_MEM_ARRAY_SIZE],
}

enum Instruction {
    Increment(i64),
    Move(i64),
    Input,
    Output,
    LoopStart(usize),
    LoopEnd(usize),
}

impl Instruction {
    fn code_len(&self) -> usize {
        match self {
            Instruction::Increment(_) => 16,
            Instruction::Move(_) => 44,
            Instruction::Input => 20,
            Instruction::Output => 20,
            Instruction::LoopStart(_) => 12,
            Instruction::LoopEnd(_) => 12,
        }
    }
}

impl Program {
    pub fn new(source: &[u8]) -> Result<Program, UnbalancedBrackets> {
        let memory = [0u8; MAX_MEM_ARRAY_SIZE];
        // The stack store [ instruction, the value is : (char index, instruction index, code index)
        let mut bracket_stack: Vec<(usize, usize, usize)> = Vec::new();
        let mut instructions: Vec<Instruction> = Vec::new();

        // Prolouge
        let mut code: Vec<u8> = vec![
            0xff, 0x43, 0x00, 0xd1, // sub	sp, sp, #16
            0xfd, 0x7b, 0x00, 0xa9, // stp	x29, x30, [sp]
            0xe8, 0x03, 0x00, 0xaa, // mov x8, x0 ; x8: self.memory
            0x09, 0x00, 0x80, 0xd2, // mov x9, #0 ; x9: current memory pointer
        ];

        let mut code_len = code.len();

        for (i, c) in source.iter().enumerate() {
            match c {
                b'+' | b'-' => {
                    let inc = if *c == b'+' { 1 } else { -1 };
                    if let Some(Instruction::Increment(v)) = instructions.last_mut() {
                        *v += inc;
                        if *v == 0 {
                            instructions.pop();
                        }
                        continue;
                    }
                    let ix = Instruction::Increment(inc);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'>' | b'<' => {
                    let step = if *c == b'>' { 1 } else { -1 };
                    if let Some(Instruction::Move(v)) = instructions.last_mut() {
                        *v += step;
                        if *v == 0 {
                            instructions.pop();
                        }
                        continue;
                    }
                    let ix = Instruction::Move(step);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'.' => {
                    let ix = Instruction::Output;
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b',' => {
                    let ix = Instruction::Input;
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b'[' => {
                    bracket_stack.push((i, instructions.len(), code_len));
                    let ix = Instruction::LoopStart(0);
                    code_len += ix.code_len();
                    instructions.push(ix);
                }
                b']' => {
                    if let Some((_, ix_i, code_i)) = bracket_stack.pop() {
                        instructions[ix_i] = Instruction::LoopStart(code_len);
                        let ix = Instruction::LoopEnd(code_i);
                        code_len += ix.code_len();
                        instructions.push(ix);
                    } else {
                        return Err(UnbalancedBrackets(']', i));
                    }
                }
                _ => continue,
            }
        }

        if !bracket_stack.is_empty() {
            return Err(UnbalancedBrackets('[', bracket_stack.pop().unwrap().0));
        }

        for ix in instructions {
            match ix {
                Instruction::Increment(inc) => {
                    let inc = inc as u32 & 0x0fff;
                    let add_ix = (0x110 << 20 | inc << 10 | 0x14a).to_le_bytes();
                    code.write_all(&[
                        0x0b, 0x01, 0x09, 0x8b, // add	x11, x8, x9
                        0x6a, 0x01, 0x40, 0x39, // ldrb	w10, [x11]
                        add_ix[0], add_ix[1], add_ix[2], add_ix[3], // add	w10, w10, #count
                        0x6a, 0x01, 0x00, 0x39, // strb	w10, [x11]
                    ])
                    .unwrap();
                }
                Instruction::Move(step) => {
                    let mut h_mask = 0xd28 << 20;
                    let mut im16 = (step as u32 & 0x0fff) << 5;
                    if step < 0 {
                        h_mask = 0x928 << 20;
                        im16 = (!step as u32 & 0x0fff) << 5;
                    }
                    let mov_ix = (h_mask | im16 | 0x0d).to_le_bytes();
                    code.write_all(&[
                        mov_ix[0], mov_ix[1], mov_ix[2], mov_ix[3], // mov x13, #step
                        0xaa, 0x34, 0x85, 0xd2, // mov	x10, #10661
                        0x2b, 0x01, 0x0d, 0x8b, // add	x11, x9, x13
                        0xaa, 0xe2, 0xac, 0xf2, // movk	x10, #26389, lsl #16
                        0x6b, 0xe1, 0x2e, 0x91, // add	x11, x11, #3000
                        0xea, 0xc3, 0xc7, 0xf2, // movk	x10, #15903, lsl #32
                        0x0d, 0x77, 0x81, 0x52, // mov	w13, #3000
                        0x6a, 0xd8, 0xf5, 0xf2, // movk	x10, #44739, lsl #48
                        0x6a, 0x7d, 0xca, 0x9b, // umulh	x10, x11, x10
                        0x4a, 0xfd, 0x4b, 0xd3, // lsr	x10, x10, #11
                        0x49, 0xad, 0x0d, 0x9b, // msub	x9, x10, x13, x11
                    ])
                    .unwrap();
                }
                Instruction::Input => {
                    code.write_all(&[
                        0x20, 0x00, 0x80, 0x52, // mov w0,  #1 ; fd: stdin
                        0x01, 0x01, 0x09, 0x8b, // add x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov x2,  #1 ; input len, only 1 number
                        0x70, 0x00, 0x80, 0x52, // mov w16, #3 ; read system call number
                        0x01, 0x10, 0x00, 0xd4, // svc #0x80
                    ])
                    .unwrap()
                }
                Instruction::Output => {
                    code.write_all(&[
                        0x00, 0x00, 0x80, 0x52, // mov	w0,  #0 ; fd: stdout
                        0x01, 0x01, 0x09, 0x8b, // add  x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov	x2,  #1 ; output len, only 1 number
                        0x90, 0x00, 0x80, 0x52, // mov	w16, #4 ; write system call number
                        0x01, 0x10, 0x00, 0xd4, // svc	#0x80    ;  syscall
                    ])
                    .unwrap()
                }
                Instruction::LoopStart(end) => {
                    let start = code.len();
                    let offset = (((end - start + 4) as u32) >> 2) & 0x7ffff;
                    let b_eq = (0x54 << 24 | offset << 5).to_le_bytes();
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	 w10, [x8, x9]
                        0x5f, 0x01, 0x0a, 0x6a, // tst	 w10, xzr
                        b_eq[0], b_eq[1], b_eq[2], b_eq[3], // b.eq	 <end>
                    ])
                    .unwrap();
                }
                Instruction::LoopEnd(start) => {
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
            }
        }

        // Epilouge
        code.write_all(&[
            0xfd, 0x7b, 0x40, 0xa9, // ldp	x29, x30, [sp]
            0xff, 0x43, 0x00, 0x91, // add	sp,  sp,  #16
            0xc0, 0x03, 0x5f, 0xd6, // ret
        ])
        .unwrap();

        // code.chunks(4).for_each(|chunk| {
        //     println!(
        //         "{:08x} ",
        //         u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]])
        //     );

        Ok(Program { code, memory })
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

fn get_os_err() -> std::io::Error {
    unsafe { std::io::Error::from_raw_os_error(*libc::__error()) }
}
