#!/bin/sh

set -e

cargo objcopy --bin rust-meetup-simple-clock --release -- -O binary ./target/rust-meetup-simple-clock.bin

openocd -f interface/stlink.cfg -f target/stm32f1x.cfg -f flash.cfg
