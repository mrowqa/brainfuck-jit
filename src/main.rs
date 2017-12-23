extern "C" {
    fn getchar() -> i32;
    fn putchar(ch: i32) -> i32;
    fn mprotect(addr: *mut u8, len: u32, prot: i32) -> i32;
    fn VirtualProtect(addr: *mut u8, len: u32, prot: i32, old_prot: *mut i32) -> i32;
}

const PAGE_SIZE: usize = 4096;

fn main() {
    let mut mem: Vec<u8> = Vec::with_capacity(PAGE_SIZE * 2);
    let ptr = mem.as_mut_ptr();
    unsafe {
        let base = ptr.offset((PAGE_SIZE - ptr as usize % PAGE_SIZE) as isize);
        println!("{:?} -> {:?}", ptr, base);
        // mprotect(base, PAGE_SIZE as u32, 7);
        let mut old_prot: i32 = 0;
        let res = VirtualProtect(ptr, PAGE_SIZE as u32, 0x40, &mut old_prot as *mut i32);
        println!("res: {:?}, old_prot: {:?}", res, old_prot);

        *ptr = 0xC3;
        let foo: fn() = std::mem::transmute(ptr);
        foo();
        println!("i have survived!");
    }

    // return;
    // let mut c: i32 = 0;
    // unsafe {
    //     while c != '.' as i32 {
    //         c = getchar();
    //         putchar(c);
    //     }
    // }
}
