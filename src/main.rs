use std::env;

use gameboy_rs::Gameboy;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() != 2 {
        println!("Usage: cargo run <filename>");
        return;
    }

    let mut gameboy = match Gameboy::load_file(&args[1]) {
        Ok(gameboy) => gameboy,
        Err(err) => {
            println!("Error loading file: {}", err);
            return;
        }
    };

    if let Err(err) = gameboy.run() {
        println!("Error running emulator: {}", err);
    }
}
