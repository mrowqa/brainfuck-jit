extern crate bf_jit;

use std::env;
use std::process::exit;
use std::fs::File;
use std::io::prelude::*;

fn main() {
    let args: Vec<_> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} file.bf", args[0]);
        exit(1);
    }

    let mut f = File::open(&args[1]).expect("file not found");
    let mut contents = String::new();
    f.read_to_string(&mut contents)
        .expect("something went wrong reading the file");

    bf_jit::run(&contents);
}
