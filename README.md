# gameboy-rs

A simple Game Boy emulator written in Rust.

The project currently includes a CPU core, memory handling, timer and interrupt
behavior, PPU rendering, a small windowed runner, tests, and a throughput
benchmark. It is intended as a straightforward emulator project rather than a
fully polished end-user application.

## Running

Provide your own Game Boy ROM:

```sh
cargo run -- path/to/rom.gb
```

ROM files are not included in this repository.
