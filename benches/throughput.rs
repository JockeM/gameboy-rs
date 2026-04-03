use std::env;
use std::fs;
use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use gameboy_rs::Gameboy;

const CPU_CLOCK_HZ: f64 = 4_194_304.0;
const DEFAULT_FRAMES: u64 = 600;
const DEFAULT_WARMUP_FRAMES: u64 = 60;

struct Config {
    frames: u64,
    warmup_frames: u64,
    rom_path: Option<PathBuf>,
}

struct Workload {
    name: &'static str,
    description: &'static str,
    rom: fn() -> Vec<u8>,
}

struct Measurement {
    frames: u64,
    cycles: u64,
    elapsed: Duration,
}

fn main() {
    let config = Config::from_args();

    if let Some(rom_path) = &config.rom_path {
        run_rom_benchmark(&config, rom_path);
        return;
    }

    let workloads = [
        Workload {
            name: "idle-loop",
            description: "tight NOP/JR loop, mostly CPU dispatch and timer/PPU stepping",
            rom: idle_loop_rom,
        },
        Workload {
            name: "register-alu",
            description: "register-heavy ALU loop with no memory traffic after setup",
            rom: register_alu_rom,
        },
        Workload {
            name: "vram-write",
            description: "VRAM write loop that also exercises scanline rendering",
            rom: vram_write_rom,
        },
    ];

    println!(
        "Running {} measured frames per workload after {} warmup frames\n",
        config.frames, config.warmup_frames
    );
    println!(
        "{:<14} {:>12} {:>14} {:>13}  workload",
        "benchmark", "frames/s", "cycles/s", "realtime"
    );

    for workload in workloads {
        let mut gameboy = Gameboy::load(&(workload.rom)());

        run_frames(&mut gameboy, config.warmup_frames);
        let measurement = measure_frames(&mut gameboy, config.frames);

        let elapsed_secs = measurement.elapsed.as_secs_f64();
        let frames_per_sec = measurement.frames as f64 / elapsed_secs;
        let cycles_per_sec = measurement.cycles as f64 / elapsed_secs;
        let realtime = cycles_per_sec / CPU_CLOCK_HZ;

        println!(
            "{:<14} {:>12.1} {:>14.0} {:>12.2}x  {}",
            workload.name, frames_per_sec, cycles_per_sec, realtime, workload.description
        );

        black_box(gameboy);
    }
}

impl Config {
    fn from_args() -> Self {
        let mut config = Self {
            frames: DEFAULT_FRAMES,
            warmup_frames: DEFAULT_WARMUP_FRAMES,
            rom_path: None,
        };

        let mut args = env::args().skip(1);
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--bench" => {}
                "--frames" => config.frames = parse_u64_arg("--frames", args.next()),
                "--warmup-frames" => {
                    config.warmup_frames = parse_u64_arg("--warmup-frames", args.next())
                }
                "--rom" => config.rom_path = Some(parse_path_arg("--rom", args.next())),
                "--help" | "-h" => {
                    print_help();
                    std::process::exit(0);
                }
                _ => {
                    eprintln!("Unknown argument: {arg}");
                    print_help();
                    std::process::exit(2);
                }
            }
        }

        config
    }
}

fn parse_u64_arg(name: &str, value: Option<String>) -> u64 {
    let value = value.unwrap_or_else(|| {
        eprintln!("Missing value for {name}");
        std::process::exit(2);
    });

    value.parse().unwrap_or_else(|_| {
        eprintln!("Invalid integer for {name}: {value}");
        std::process::exit(2);
    })
}

fn parse_path_arg(name: &str, value: Option<String>) -> PathBuf {
    value.map(PathBuf::from).unwrap_or_else(|| {
        eprintln!("Missing value for {name}");
        std::process::exit(2);
    })
}

fn print_help() {
    println!(
        "Usage: cargo bench --bench throughput -- [--frames N] [--warmup-frames N] [--rom path/to/game.gb]\n\
         Defaults: --frames {DEFAULT_FRAMES} --warmup-frames {DEFAULT_WARMUP_FRAMES}"
    );
}

fn run_rom_benchmark(config: &Config, rom_path: &PathBuf) {
    let rom = fs::read(rom_path).unwrap_or_else(|err| {
        eprintln!("Failed to read ROM {}: {err}", rom_path.display());
        std::process::exit(1);
    });
    let mut gameboy = Gameboy::load(&rom);

    run_frames(&mut gameboy, config.warmup_frames);
    let measurement = measure_frames(&mut gameboy, config.frames);

    let elapsed_secs = measurement.elapsed.as_secs_f64();
    let frames_per_sec = measurement.frames as f64 / elapsed_secs;
    let cycles_per_sec = measurement.cycles as f64 / elapsed_secs;
    let realtime = cycles_per_sec / CPU_CLOCK_HZ;

    println!(
        "Running ROM {} for {} measured frames after {} warmup frames\n",
        rom_path.display(),
        config.frames,
        config.warmup_frames
    );
    println!(
        "{:<14} {:>12} {:>14} {:>13}",
        "benchmark", "frames/s", "cycles/s", "realtime"
    );
    println!(
        "{:<14} {:>12.1} {:>14.0} {:>12.2}x",
        "rom", frames_per_sec, cycles_per_sec, realtime
    );

    black_box(gameboy);
}

fn run_frames(gameboy: &mut Gameboy, frames: u64) {
    for _ in 0..frames {
        gameboy.run_frame();
    }
}

fn measure_frames(gameboy: &mut Gameboy, frames: u64) -> Measurement {
    let start_cycles = gameboy.cycles;
    let start = Instant::now();

    run_frames(gameboy, frames);

    Measurement {
        frames,
        cycles: gameboy.cycles - start_cycles,
        elapsed: start.elapsed(),
    }
}

fn base_rom(program: &[u8]) -> Vec<u8> {
    let mut rom = vec![0; 0x150];
    rom[0x100..0x100 + program.len()].copy_from_slice(program);
    rom[0x134..0x143].copy_from_slice(b"THROUGHPUTBENCH");
    rom
}

fn idle_loop_rom() -> Vec<u8> {
    base_rom(&[
        0x00, // NOP
        0x00, // NOP
        0x18, 0xFC, // JR -4
    ])
}

fn register_alu_rom() -> Vec<u8> {
    base_rom(&[
        0x3E, 0x01, // LD A,$01
        0x06, 0x03, // LD B,$03
        0x0E, 0x05, // LD C,$05
        0x80, // ADD A,B
        0x89, // ADC A,C
        0xA8, // XOR B
        0xB1, // OR C
        0x3C, // INC A
        0x05, // DEC B
        0x04, // INC B
        0x18, 0xF7, // JR -9
    ])
}

fn vram_write_rom() -> Vec<u8> {
    base_rom(&[
        0x21, 0x00, 0x80, // LD HL,$8000
        0x3E, 0xFF, // LD A,$FF
        0x22, // LD (HL+),A
        0x22, // LD (HL+),A
        0x7C, // LD A,H
        0xFE, 0x98, // CP $98
        0x20, 0xF8, // JR NZ,-8
        0x18, 0xF1, // JR -15
    ])
}
