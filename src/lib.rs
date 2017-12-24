#[macro_use]
extern crate lazy_static;

mod vm;
pub use vm::BfJitVM;

pub fn run(code: &str) {
    let mut vm = BfJitVM::new(0x10000, 0x10000).expect("Failed to create Brainfuck JIT VM.");
    vm.compile(code);
    vm.run();
}
