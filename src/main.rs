#![no_main]
#![no_std]

extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use embedded_graphics::{fonts::Font8x16, prelude::*};
use rtfm::app;
use ssd1306::{interface::I2cInterface, mode::GraphicsMode, Builder};
use stm32f1xx_hal::{
    gpio::{
        gpiob::{PB8, PB9},
        Alternate, OpenDrain,
    },
    i2c::{BlockingI2c, DutyCycle, Mode},
    prelude::*,
    stm32::I2C1,
};

type OledDisplay = GraphicsMode<
    I2cInterface<BlockingI2c<I2C1, (PB8<Alternate<OpenDrain>>, PB9<Alternate<OpenDrain>>)>>,
>;

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut DISPLAY: OledDisplay = ();

    #[init]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc.cfgr.freeze(&mut flash.acr);

        let mut afio = device.AFIO.constrain(&mut rcc.apb2);

        let mut gpiob = device.GPIOB.split(&mut rcc.apb2);

        let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
        let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);

        let i2c = BlockingI2c::i2c1(
            device.I2C1,
            (scl, sda),
            &mut afio.mapr,
            Mode::Fast {
                frequency: 400_000,
                duty_cycle: DutyCycle::Ratio2to1,
            },
            clocks,
            &mut rcc.apb1,
            1000,
            10,
            1000,
            1000,
        );

        let mut display: GraphicsMode<_> = Builder::new().connect_i2c(i2c).into();

        display.init().expect("Failed to initialise display");
        display.flush().expect("Failed to clear display");

        hprintln!("init complete").unwrap();

        init::LateResources { DISPLAY: display }
    }

    #[idle(resources = [DISPLAY])]
    fn idle() -> ! {
        hprintln!("idle").unwrap();

        resources.DISPLAY.draw(
            Font8x16::render_str("Hello world!")
                .with_stroke(Some(1u8.into()))
                .into_iter(),
        );

        resources.DISPLAY.flush().expect("Failed to update display");

        loop {}
    }
};
