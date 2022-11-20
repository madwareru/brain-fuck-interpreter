use std::env;
use std::fs;
use std::path::Path;
use std::time::Instant;

const MANDELBROT: &[u8] = include_bytes!("src/mandelbrot.b");

fn main() {
    let out_dir = env::var_os("OUT_DIR").unwrap();
    let dest_path = Path::new(&out_dir).join("mandelbrot_generated.rs");

    let instant = Instant::now();

    let mut s = String::with_capacity(1000000);
    s += "pub fn run_mandelbrot_generated() {\n";
    s += "    let mut tape: Vec<u8> = Vec::with_capacity(0x100000); tape.resize(0x100000, 0);\n";
    s += "    let mut tape_pos = 0;\n";

    let mut iterator = MANDELBROT.iter();

    while let Some(cur_code) = iterator.next() {
        match cur_code {
            b'[' => { s += "    while tape[tape_pos] != 0 {\n"; }
            b']' => { s += "    }\n"; }
            b'+' => { s += "    tape[tape_pos] += 1;\n"; },
            b'-' => { s += "    tape[tape_pos] -= 1;\n"; },
            b'>' => { s += "    tape_pos += 1;\n"; },
            b'<' => { s += "    tape_pos -= 1;\n"; },
            b'.' => { s += "    print!(\"{}\", tape[tape_pos] as char);\n";  },
            b',' => { s += "    tape[tape_pos] = unsafe { libc::getchar() } as u8;\n"; },
            _ => ()
        }
    }
    s += &format!("print!(\"\\ncode generation took {} seconds\\n\");\n", instant.elapsed().as_secs_f32());
    s += "}\n";
    fs::write(&dest_path, &s).unwrap();
    println!("cargo:rerun-if-changed=build.rs");
}