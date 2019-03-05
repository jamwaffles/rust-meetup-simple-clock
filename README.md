# Simple clock demo

README is WIP

## Requirements

* Basic working knowledge of Rust
* A dev board with STM32F103, SSD1306 and a 5 button mini joy stick
* STLink V2 or compatible debugger
* Linux computer with a USB port. Macs should also work.
* [rustup](https://rustup.rs)

## Setup

* OpenOCD. For Debian-based systems: `apt install openocd`
* `gdb-multiarch`. For Debian-based systems: `apt install gdb-multiarch`
* `rustup target add thumbv7m-none-eabi`
* Add `add-auto-load-safe-path /absolute/path/to/repository`
