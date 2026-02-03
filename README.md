# Elaris

A NES (Nintendo Entertainment System) emulator written in Rust. Work in progress.

## Features

- **6502 CPU** – Full instruction set including undocumented opcodes; nestest-compatible
- **PPU** – Background and sprite rendering, nametable mirroring, OAM, 256×240 framebuffer
- **APU** – Pulse (×2), triangle, and noise channels; frame counter (4-step and 5-step); 44.1 kHz audio
- **Cartridge** – iNES (.nes) loading; mappers: NROM (0), MMC1 (1)
- **Controller** – Port 1 ($4016) shift-register protocol
- **Display** – 256×240 window via minifb; scaled to fit
- **Audio** – Output via rodio (default device)

## Requirements

- Rust (e.g. `rustup default stable`)

## Build & Run

```bash
cargo build --release
cargo run --release -- path/to/game.nes
```

Without a path, the emulator defaults to `test/nestest.nes` (for CPU verification).

## Usage

```text
elaris [path/to/rom.nes]
```

- **Escape** – Close the window and exit.

Controller input is wired to port 1; button state is provided by the `Controller` struct (e.g. for future GUI or key mapping).

## Nestest

To verify the CPU against [nestest](https://www.qmtpro.com/~nes/misc/nestest.html):

```bash
cargo run --release -- test/nestest.nes
```

The emulator starts at the nestest entry point (`$C000`) and runs until it hits a JAM. Compare cycle count and final state with `nestest.log` if needed.

## Project layout

| Path | Description |
|------|-------------|
| `src/main.rs` | Entry point, window, audio sink, frame loop |
| `src/lib.rs` | Crate root and module list |
| `src/bus.rs` | Memory map, PPU/APU/controller/cartridge dispatch |
| `src/cpu/` | 6502 CPU and status flags |
| `src/ppu/` | PPU timing, background, sprites, OAM, framebuffer |
| `src/apu/` | APU channels, frame counter, mixer, sample buffer |
| `src/cartridge/` | iNES loading and mappers (NROM, MMC1) |
| `src/controller.rs` | NES controller shift register |

## License

GPL-3.0-or-later. See [LICENSE.md](LICENSE.md).
