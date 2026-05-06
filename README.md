# gameboy-rs

Game Boy emulator crate in a Cargo workspace.

The project currently includes a CPU core, memory handling, timer and interrupt
behavior, PPU rendering, a small windowed runner, tests, and a throughput
benchmark.

The emulator lives in `crates/gameboy` so more crates can be added alongside it
later.

## Running

Provide your own Game Boy ROM:

```sh
cargo run -p gameboy-rs --features window -- path/to/rom.gb
```

ROM files are not included in this repository.

## Checking

```sh
cargo check --workspace --all-targets
cargo test --workspace
```
