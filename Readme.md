Brainfuck JIT VM
================

After seeing [this](https://www.youtube.com/watch?v=ApOUBBOvZDo) and still
having my strong willing to write some JIT Compiler in Rust,
I created this :) .

I didn't wanted to use some assembler nor library like keystone or asmjit.
I wanted to keep this project small and fun. And that's why it generates
such suboptimal code.

The target architecture is *Windows x64*, cause why not?
Linux is too mainstream.

Tested with `rustc 1.22.1 (05e2e1c41 2017-11-22)`
on `Microsoft Windows [Version 10.0.16299.125]`
using `examples/num-guess.bf`.

Usage:
------

```sh
$ cargo run --release examples/num-guess.bf
```
