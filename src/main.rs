#![no_main]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rt as rt;
extern crate panic_semihosting;
extern crate stm32f1xx_hal as hal;

use cortex_m_semihosting::{debug, hprintln};
use rtfm::app;

#[app(device = stm32f1xx_hal::stm32f1::stm32f103)]
const APP: () = {
    #[init]
    fn init() {
        hprintln!("init").unwrap();
    }

    #[idle]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        // End the program
        debug::exit(debug::EXIT_SUCCESS);

        loop {}
    }
};
