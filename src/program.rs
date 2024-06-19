use std::io::Write;

const MAX_MEM_ARRAY_SIZE: usize = 30000;

pub struct UnbalancedBrackets(pub char, pub usize);

pub struct Program {
    code: Vec<u8>,
    memory: [u8; MAX_MEM_ARRAY_SIZE],
}

impl Program {
    pub fn new(source: &[u8]) -> Result<Program, UnbalancedBrackets> {
        // Prolouge
        let mut code: Vec<u8> = vec![
            0xff, 0x43, 0x00, 0xd1, // sub	sp, sp, #16
            0xfd, 0x7b, 0x00, 0xa9, // stp	x29, x30, [sp]
            0xe8, 0x03, 0x00, 0xaa, // mov x8, x0 ; x8: self.memory
            0x09, 0x00, 0x80, 0xd2, // mov x9, #0 ; x9: current memory pointer
        ];
        let memory = [0u8; MAX_MEM_ARRAY_SIZE];
        let mut bracket_stack: Vec<usize> = Vec::new();

        for c in source {
            match c {
                b'+' => code
                    .write_all(&[
                        0x0b, 0x01, 0x09, 0x8b, // add	x11, x8, x9
                        0x6a, 0x01, 0x40, 0x39, // ldrb	w10, [x11]
                        0x4a, 0x05, 0x00, 0x11, // add	w10, w10, #1
                        0x6a, 0x01, 0x00, 0x39, // strb	w10, [x11]
                    ])
                    .unwrap(),
                b'-' => code
                    .write_all(&[
                        0x0b, 0x01, 0x09, 0x8b, // add	x11, x8, x9
                        0x6a, 0x01, 0x40, 0x39, // ldrb	w10, [x11]
                        0x4a, 0x05, 0x00, 0x51, // sub	w10, w10, #1
                        0x6a, 0x01, 0x00, 0x39, // strb	w10, [x11]
                    ])
                    .unwrap(),
                b'>' => code
                    .write_all(&[
                        0x2a, 0x05, 0x00, 0x91, // add	x10, x9, #1
                        0x0b, 0xa6, 0x8e, 0x52, // mov	w11, #30000
                        0x5f, 0x01, 0x0b, 0xeb, // cmp	x10, x11
                        0xe9, 0x03, 0x8a, 0x9a, // csel	x9, xzr, x10, eq
                    ])
                    .unwrap(),
                b'<' => code
                    .write_all(&[
                        0x2a, 0x05, 0x00, 0xd1, // sub	x10, x9, #1
                        0xeb, 0xa5, 0x8e, 0x52, // mov	w11, #29999
                        0x3f, 0x01, 0x09, 0xea, // tst	x9,  x9
                        0x69, 0x01, 0x8a, 0x9a, // csel	x9,  x11, x10, eq
                    ])
                    .unwrap(),
                b'.' => code
                    .write_all(&[
                        0x00, 0x00, 0x80, 0x52, // mov	w0,  #0 ; fd: stdout
                        0x01, 0x01, 0x09, 0x8b, // add  x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov	x2,  #1 ; output len, only 1 number
                        0x90, 0x00, 0x80, 0x52, // mov	w16, #4 ; write system call number
                        0x01, 0x10, 0x00, 0xd4, // svc	#0x80    ;  syscall
                    ])
                    .unwrap(),
                b',' => code
                    .write_all(&[
                        0x20, 0x00, 0x80, 0x52, // mov w0,  #1 ; fd: stdin
                        0x01, 0x01, 0x09, 0x8b, // add x1,  x8, x9
                        0x22, 0x00, 0x80, 0xd2, // mov x2,  #1 ; input len, only 1 number
                        0x70, 0x00, 0x80, 0x52, // mov w16, #3 ; read system call number
                        0x01, 0x10, 0x00, 0xd4, // svc #0x80
                    ])
                    .unwrap(),
                b'[' => {
                    bracket_stack.push(code.len());
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	 w10, [x8, x9]
                        0x5f, 0x01, 0x0a, 0x6a, // tst	 w10, xzr
                        0x00, 0x00, 0x00, 0x54, // b.eq	 0x00  ; Change the offset later
                    ])
                    .unwrap();
                }
                b']' => {
                    let start = bracket_stack.pop();
                    if start.is_none() {
                        return Err(UnbalancedBrackets(']', code.len()));
                    }

                    let start = start.unwrap();
                    let end = code.len();
                    code.write_all(&[
                        0x0a, 0x69, 0x69, 0x38, // ldrb	 w10, [x8, x9]
                        0x5f, 0x01, 0x0a, 0x6a, // tst	 w10, w10
                    ])
                    .unwrap();

                    // Append instruction b.ne	<start>
                    let offset = ((((start - end + 4) as i32) >> 2) as u32) & 0x7ffff;
                    let branch_ix = 0x54 << 24 | offset << 5 | 0x1;
                    code.write_all(&branch_ix.to_le_bytes()).unwrap();

                    // Change branch position of b.eq <end>
                    let offset = (((end - start + 4) as u32) >> 2) & 0x7ffff;
                    let branch_ix = 0x54 << 24 | offset << 5;
                    code[start + 8..start + 12].copy_from_slice(&branch_ix.to_le_bytes());
                }
                _ => continue,
            };
        }

        if !bracket_stack.is_empty() {
            return Err(UnbalancedBrackets('[', bracket_stack.pop().unwrap()));
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
        // });

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
