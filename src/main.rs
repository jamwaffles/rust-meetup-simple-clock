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
    rtc::Rtc,
    stm32::I2C1,
};

type OledDisplay = GraphicsMode<
    I2cInterface<BlockingI2c<I2C1, (PB8<Alternate<OpenDrain>>, PB9<Alternate<OpenDrain>>)>>,
>;

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut RTC_DEVICE: Rtc = ();
    static mut DISPLAY: OledDisplay = ();

    #[init]
    fn init() -> init::LateResources {
        hprintln!("init").unwrap();

        let mut flash = device.FLASH.constrain();
        let mut rcc = device.RCC.constrain();

        let clocks = rcc
            .cfgr
            .use_hse(8.mhz())
            .sysclk(72.mhz())
            .pclk1(36.mhz())
            .freeze(&mut flash.acr);

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

        let mut pwr = device.PWR;
        let mut backup_domain = rcc.bkp.constrain(device.BKP, &mut rcc.apb1, &mut pwr);

        // Enable RTC interrupt
        device.RTC.crh.write(|w| w.secie().set_bit());
        let mut rtc = Rtc::rtc(device.RTC, &mut backup_domain);

        hprintln!("init complete").unwrap();

        init::LateResources {
            DISPLAY: display,
            RTC_DEVICE: rtc,
        }
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

    #[interrupt(priority = 3, resources = [RTC_DEVICE])]
    fn RTC() {
        hprintln!("tick {}", resources.RTC_DEVICE.seconds()).unwrap();
    }
};
