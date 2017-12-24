extern "C" {
    fn getchar() -> i32;
    fn putchar(ch: i32) -> i32;
}

mod jitmem;
use self::jitmem::JitMemory;
use std::collections::HashMap;

// Possible optimizations:
// * multiple + and - as add/sub byte [rsi], X
// * we can save getchar and putchar in some registers in the prolog
//   and then we can call them directly (or move to rax if necessary)
lazy_static! {
    static ref INSTR_TO_BIN_CODE: HashMap<char, &'static [u8]> = {
        let mut m: HashMap<char, &[u8]> = HashMap::new();
        m.insert('d', &[0xCC]); // software breakpoint
        m.insert('p', &[0x56]); // push rsi
        m.insert('q', &[0x48, 0xbe, 0, 0, 0, 0, 0, 0, 0, 0]); // mov rsi, addr
        m.insert('P', &[0x5e]); // pop rsi
        m.insert('r', &[0xc3]); // ret
        m.insert('>', &[0x48, 0xff, 0xc6]); // inc rsi
        m.insert('<', &[0x48, 0xff, 0xce]); // dec rsi
        m.insert('+', &[0xfe, 0x06]); // inc byte [rsi]
        m.insert('-', &[0xfe, 0x0e]); // dec byte [rsi]
        m.insert('[', &[
            0x80, 0x3e, 0x00, // cmp byte [rsi], al
            0x0f, 0x84, 0, 0, 0, 0 // je addr (rel, 32 bit)
        ]);
        m.insert(']', &[
            0x80, 0x3e, 0x00, // cmp byte [rsi], al
            0x0f, 0x85, 0, 0, 0, 0 // jne addr (rel, 32 bit)
        ]);
        m.insert('.', &[
            0x0f, 0xb6, 0x0e, // movzx ecx, byte [rsi]
            0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, // mov rax, putchar
            0xff, 0xd0, // call rax
        ]); 
        m.insert(',', &[
            0x48, 0xb8, 0, 0, 0, 0, 0, 0, 0, 0, // mov rax, getchar
            0xff, 0xd0, // call rax
            0x88, 0x06, // mov byte [rsi], al
        ]);
        m
    };
}

pub struct BfJitVM {
    code_mem: JitMemory,
    data_mem: Vec<u8>,
}

impl BfJitVM {
    pub fn new(code_size: usize, data_size: usize) -> Option<BfJitVM> {
        let mut jit_mem = JitMemory::alloc(code_size)?;
        jit_mem.write_at(&mut 0, INSTR_TO_BIN_CODE[&'r']);
        let data_mem = vec![0; data_size];

        Some(BfJitVM {
            code_mem: jit_mem,
            data_mem: data_mem,
        })
    }

    pub fn compile(&mut self, code: &str) -> bool {
        let code_bytes = code.as_bytes();
        if !self.check_before_compilation(code_bytes) {
            return false;
        }
        self.compile_helper(code_bytes);
        true
    }

    pub fn run(&mut self) {
        // zero out the data memory
        for it in self.data_mem.iter_mut() {
            *it = 0;
        }

        let jit_function = self.code_mem.as_function();
        println!("JitVM: Running code from addr 0x{:x}", jit_function as u64);
        println!("JitVM: VM memory at 0x{:x}", self.data_mem.as_ptr() as u64);
        println!("-------------------");
        jit_function();
    }

    // --------------- compiler -------------------
    fn check_before_compilation(&self, code: &[u8]) -> bool {
        let mut required_code_mem = 0;
        let mut opened_loops = 0;

        for ch in code {
            let chh = *ch as char;
            match chh {
                '<' | '>' | '+' | '-' | ',' | '.' => {
                    required_code_mem += INSTR_TO_BIN_CODE[&chh].len();
                }
                '[' => {
                    opened_loops += 1;
                    required_code_mem += INSTR_TO_BIN_CODE[&chh].len();
                }
                ']' => {
                    if opened_loops == 0 {
                        println!("JitCompiler: Error: found ']' without corresponding '['.");
                        return false;
                    }
                    opened_loops -= 1;
                    required_code_mem += INSTR_TO_BIN_CODE[&chh].len();
                }
                _ => {}
            }
        }

        if opened_loops > 0 {
            println!("JitCompiler: Error: too many ']'.");
            return false;
        }

        // note: this var occurs also in compile_helper!
        let stack_alignment_blocks = 5;
        // prolog
        required_code_mem += INSTR_TO_BIN_CODE[&'p'].len() * stack_alignment_blocks;
        required_code_mem += INSTR_TO_BIN_CODE[&'q'].len();
        // epilog
        required_code_mem += INSTR_TO_BIN_CODE[&'P'].len() * stack_alignment_blocks;
        required_code_mem += INSTR_TO_BIN_CODE[&'r'].len();
        let vm_code_buffer_size = self.code_mem.size();
        if required_code_mem > vm_code_buffer_size {
            println!("JitCompiler: Error: code requires {} bytes, but VM has a buffer of {} \
                      bytes.",
                     required_code_mem,
                     vm_code_buffer_size);
            return false;
        }

        println!("JitCompiler: Warning: did not validate if VM memory buffer is big enough and \
                  if program accesses memory beyond its boundaries.");
        println!("JitCompiler: Warning: assuming that getchar and putchar always succeeds.");
        true
    }

    fn compile_helper(&mut self, code: &[u8]) {
        let mut ip = 0;
        // for debugging:
        //self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&'d']);
        let stack_alignment_blocks = 5;
        for _ in 0..stack_alignment_blocks {
            // space on the stack for calling putchar/getchar + preserving rsi
            self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&'p']);
        }
        self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&'q']);
        let addr_size = 8;
        self.code_mem.patch_addr_u64(ip - addr_size, self.data_mem.as_ptr() as u64);
        let (mut ip, chars_processed) = self.compile_loop_body(code, ip);
        assert_eq!(chars_processed, code.len());
        for _ in 0..stack_alignment_blocks {
            self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&'P']);
        }
        self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&'r']);
        println!("JitCompiler: compilation resulted in {} bytes.", ip);
        println!("-------------------");
    }

    // returns (new ip, code chars processed)
    fn compile_loop_body(&mut self, code: &[u8], begin_ip: usize) -> (usize, usize) {
        let mut ip = begin_ip;
        let mut chars_processed = 0;
        while chars_processed < code.len() {
            let ch = code[chars_processed] as char;
            match ch {
                '<' | '>' | '+' | '-' => {
                    self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&ch]);
                }
                '.' => {
                    self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&ch]);
                    let putchar_addr_offset = 10;
                    self.code_mem.patch_addr_u64(ip - putchar_addr_offset, putchar as u64);
                }
                ',' => {
                    self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&ch]);
                    let getchar_addr_offset = 12;
                    self.code_mem.patch_addr_u64(ip - getchar_addr_offset, getchar as u64);
                }
                '[' => {
                    self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&ch]);
                    let addr_to_patch = ip - 4;
                    let (new_ip, new_cp) = self.compile_loop_body(&code[chars_processed + 1..], ip);
                    let offset = new_ip - ip;
                    assert!(offset > 0 && offset < (1 << 31));
                    self.code_mem.patch_addr_i32(addr_to_patch, offset as i32);
                    ip = new_ip;
                    chars_processed += new_cp;
                }
                ']' => {
                    self.code_mem.write_at(&mut ip, INSTR_TO_BIN_CODE[&ch]);
                    let addr_to_patch = ip - 4;
                    let offset = ip - begin_ip;
                    assert!(offset > 0 && offset < (1 << 31));
                    self.code_mem.patch_addr_i32(addr_to_patch, -(offset as i32));
                }
                _ => {}
            }

            chars_processed += 1;
            if ch == ']' {
                break;
            }
        }

        (ip, chars_processed)
    }
}
