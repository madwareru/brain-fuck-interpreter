use proc_macro_bf::bf;

use std::time::Instant;

#[cfg(feature = "use_codegen")]
mod codegen {
    include!(concat!(env!("OUT_DIR"), "/mandelbrot_generated.rs"));
}

#[cfg(not(feature = "use_codegen"))]
const MANDELBROT: &[u8] = include_bytes!("mandelbrot.b");

bf!(bf_hello, "++++++++[>++++[>++>+++>+++>+<<<<-]>+>+>->>+[<]<-]>>.>---.+++++++..+++.>>.<-.<.+++.------.--------.>>+.>++.");

#[cfg(not(feature = "use_codegen"))]
#[inline(always)]
fn get_char_impl() -> u8 {
    let c = unsafe { libc::getchar() };
    c as u8
}

#[cfg(not(feature = "use_codegen"))]
fn run(code: &[u8]) {
    let mut tape = Vec::with_capacity(0x100000);
    tape.resize(0x100000, 0);
    let mut code = code;
    let mut tape_pos = 0;
    run_step(&mut code, &mut tape, &mut tape_pos, false);
}

#[cfg(not(feature = "use_codegen"))]
fn run_step(code: &mut &[u8], tape: &mut Vec<u8>, tape_pos: &mut usize, skip: bool) -> bool {
    while (*code).len() > 0 {
        match { (*code)[0] } {
            b'[' => {
                *code = &(*code)[1..];
                let old_code = *code;
                while run_step(code, tape, tape_pos, tape[*tape_pos] == 0 ) {
                    *code = old_code;
                }
            },
            b']' => { return tape[*tape_pos] != 0 },
            code if !skip => {
                match code {
                    b'+' => { tape[*tape_pos] += 1; },
                    b'-' => { tape[*tape_pos] -= 1; },
                    b'>' => { *tape_pos += 1 },
                    b'<' => { *tape_pos -= 1; },
                    b'.' => { print!("{}", tape[*tape_pos] as char); },
                    b',' => { tape[*tape_pos] = get_char_impl(); },
                    _ => ()
                }
            },
            _ => ()
        }
        *code = &(*code)[1..];
    }
    false
}

fn main() {
    bf_hello();
    let instant = Instant::now();

    #[cfg(feature = "use_codegen")]
    {
        codegen::run_mandelbrot_generated();
    }
    #[cfg(not(feature = "use_codegen"))]
    {
        run(MANDELBROT);
    }
    let elapsed = instant.elapsed().as_secs_f32();
    println!("time: {elapsed} seconds");
}
