use std::ops::{Index, IndexMut};
use std::mem;

extern "C" {
    fn VirtualProtect(addr: *mut u8, len: usize, prot: i32, old_prot: *mut i32) -> i32;
}

pub struct JitMemory {
    mem: Vec<u8>,
}

impl JitMemory {
    pub fn alloc(bytes: usize) -> Option<JitMemory> {
        let mut vec = vec![0; bytes];
        let mut old_prot = 0;
        let execute_read_write = 0x40;
        let res;
        unsafe {
            res = VirtualProtect(vec.as_mut_ptr(),
                                 bytes,
                                 execute_read_write,
                                 &mut old_prot as *mut i32);
        }

        if res != 0 {
            Some(JitMemory { mem: vec })
        } else {
            None
        }
    }

    pub fn write_at(&mut self, offset: &mut usize, data: &[u8]) {
        assert!(*offset + data.len() <= self.mem.len());

        for i in 0..data.len() {
            self.mem[*offset + i] = data[i];
        }

        *offset += data.len();
    }

    pub fn patch_addr_u64(&mut self, offset: usize, addr: u64) {
        assert!(offset + 8 <= self.mem.len());
        unsafe {
            *(&mut self.mem[offset] as *mut u8 as *mut u64) = addr;
        }
    }

    pub fn patch_addr_i32(&mut self, offset: usize, addr: i32) {
        assert!(offset + 4 <= self.mem.len());
        unsafe {
            *(&mut self.mem[offset] as *mut u8 as *mut i32) = addr;
        }
    }

    pub fn as_function(&self) -> fn() {
        unsafe { mem::transmute(self.mem.as_ptr()) }
    }

    pub fn size(&self) -> usize {
        self.mem.len()
    }
}

impl Index<usize> for JitMemory {
    type Output = u8;

    fn index(&self, _index: usize) -> &u8 {
        &self.mem[_index]
    }
}

impl IndexMut<usize> for JitMemory {
    fn index_mut(&mut self, _index: usize) -> &mut u8 {
        &mut self.mem[_index]
    }
}
