mod gameboy;
mod registers;

use gameboy::Gameboy;

use std::env;
use std::fs::File;
use std::io::Read;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: cargo run <filename>");
        return;
    }

    let file_path = &args[1];

    let mut file = match File::open(file_path) {
        Ok(file) => file,
        Err(err) => {
            println!("Error opening file: {}", err);
            return;
        }
    };

    let mut buffer = Vec::new();
    if let Err(err) = file.read_to_end(&mut buffer) {
        println!("Error reading file: {}", err);
        return;
    }

    let mut gameboy = Gameboy::load(buffer.as_slice());
    gameboy.run();
}
