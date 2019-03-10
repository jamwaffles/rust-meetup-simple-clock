#![no_main]
#![no_std]

extern crate panic_semihosting;

use core::f32;
use core::fmt::{self, Write};
use cortex_m_semihosting::hprintln;
use embedded_graphics::{
    fonts::Font8x16,
    pixelcolor::PixelColorU8,
    prelude::*,
    primitives::{Circle, Line},
};
use heapless::consts::*;
use heapless::String;
use libm::F32Ext;
use rtfm::app;
use ssd1306::{interface::I2cInterface, mode::GraphicsMode, Builder};
use stm32f1xx_hal::{
    gpio::{
        gpioa::{PA0, PA1, PA2, PA3, PA4},
        gpiob::{PB8, PB9},
        Alternate, Input, OpenDrain, PullDown,
    },
    i2c::{BlockingI2c, DutyCycle, Mode},
    prelude::*,
    rtc::Rtc,
    stm32::I2C1,
    timer,
};

type OledDisplay = GraphicsMode<
    I2cInterface<BlockingI2c<I2C1, (PB8<Alternate<OpenDrain>>, PB9<Alternate<OpenDrain>>)>>,
>;

pub struct Buttons {
    up: PA0<Input<PullDown>>,
    down: PA3<Input<PullDown>>,
    left: PA2<Input<PullDown>>,
    right: PA4<Input<PullDown>>,
    center: PA1<Input<PullDown>>,
}

pub struct ButtonState {
    up: bool,
    down: bool,
    left: bool,
    right: bool,
    center: bool,
}

const TWO_PI: f32 = 2.0_f32 * f32::consts::PI;

fn clock_face(display: &mut OledDisplay) {
    let cx = 64;
    let cy = 32;
    let r: i32 = 31;
    let tick_len = 4;
    // Offset from outer edge of circle
    let tick_offs = 3;

    // Draw hour ticks
    for hour in 0..=12 {
        let phase = (hour as f32 / 12.0) * TWO_PI;
        let (phase_sin, phase_cos) = phase.sin_cos();

        let x0 = cx as f32 + ((r - tick_offs) as f32 * phase_sin);
        let y0 = cy as f32 + ((r - tick_offs) as f32 * phase_cos);

        let x1 = cx as f32 + ((r - tick_offs - tick_len) as f32 * phase_sin);
        let y1 = cy as f32 + ((r - tick_offs - tick_len) as f32 * phase_cos);

        display.draw(
            Line::new(
                Coord::new(x0 as i32, y0 as i32),
                Coord::new(x1 as i32, y1 as i32),
            )
            .with_stroke(Some(1u8.into()))
            .into_iter(),
        );
    }

    // Outline circle
    display.draw(
        Circle::new(Coord::new(cx, cy), r as u32)
            .with_stroke(Some(1u8.into()))
            .into_iter(),
    );
}

fn clock_hands(display: &mut OledDisplay, seconds: u32) {
    let (hour_sin, hour_cos) = ((seconds as f32 / 86400.0) * TWO_PI).sin_cos();
    let (minute_sin, minute_cos) = ((seconds as f32 / 3660.0) * TWO_PI).sin_cos();
    let (second_sin, second_cos) = ((seconds as f32 / 60.0) * TWO_PI).sin_cos();

    let cx = 64.0;
    let cy = 32.0;
    let r = 31.0;
    let hour_hand_len = r - 15.0;
    let minute_hand_len = r - 10.0;
    let second_hand_len = r - 5.0;

    // Hour hand
    display.draw(
        Line::new(
            Coord::new(cx as i32, cy as i32),
            Coord::new(
                (cx - hour_hand_len * hour_sin) as i32,
                (cy + hour_hand_len * hour_cos) as i32,
            ),
        )
        .with_stroke(Some(1u8.into()))
        .into_iter(),
    );

    // Minute hand
    display.draw(
        Line::new(
            Coord::new(cx as i32, cy as i32),
            Coord::new(
                (cx - minute_hand_len * minute_sin) as i32,
                (cy + minute_hand_len * minute_cos) as i32,
            ),
        )
        .with_stroke(Some(1u8.into()))
        .into_iter(),
    );

    // Second hand
    display.draw(
        Line::new(
            Coord::new(cx as i32, cy as i32),
            Coord::new(
                (cx - second_hand_len * second_sin) as i32,
                (cy + second_hand_len * second_cos) as i32,
            ),
        )
        .with_stroke(Some(1u8.into()))
        .into_iter(),
    );
}

#[app(device = stm32f1xx_hal::stm32)]
const APP: () = {
    static mut RTC_DEVICE: Rtc = ();
    static mut DISPLAY: OledDisplay = ();
    static mut BUTTONS: Buttons = ();
    static mut BUTTON_STATE: ButtonState = ButtonState {
        up: false,
        down: false,
        left: false,
        right: false,
        center: false,
    };

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
        let mut gpioa = device.GPIOA.split(&mut rcc.apb2);

        let up = gpioa.pa0.into_pull_down_input(&mut gpioa.crl);
        let down = gpioa.pa3.into_pull_down_input(&mut gpioa.crl);
        let left = gpioa.pa2.into_pull_down_input(&mut gpioa.crl);
        let right = gpioa.pa4.into_pull_down_input(&mut gpioa.crl);
        let center = gpioa.pa1.into_pull_down_input(&mut gpioa.crl);

        let buttons = Buttons {
            up,
            down,
            left,
            right,
            center,
        };

        let mut timer = timer::Timer::tim1(device.TIM1, 1.khz(), clocks, &mut rcc.apb2);
        timer.listen(timer::Event::Update);

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

        clock_face(&mut display);

        display.flush().expect("Failed to update display");

        let mut pwr = device.PWR;
        let mut backup_domain = rcc.bkp.constrain(device.BKP, &mut rcc.apb1, &mut pwr);

        // Enable RTC interrupt
        device.RTC.crh.write(|w| w.secie().set_bit());

        let mut rtc = Rtc::rtc(device.RTC, &mut backup_domain);

        hprintln!("init complete").unwrap();

        init::LateResources {
            DISPLAY: display,
            RTC_DEVICE: rtc,
            BUTTONS: buttons,
        }
    }

    #[interrupt(priority = 3, resources = [DISPLAY, RTC_DEVICE])]
    fn RTC() {
        resources.DISPLAY.clear();

        clock_face(&mut resources.DISPLAY);
        clock_hands(&mut resources.DISPLAY, resources.RTC_DEVICE.seconds());

        resources.DISPLAY.flush().expect("Failed to update display");
    }

    #[interrupt(priority = 4, resources = [DISPLAY, BUTTONS, BUTTON_STATE])]
    fn TIM1_UP() {
        *resources.BUTTON_STATE = ButtonState {
            up: resources.BUTTONS.up.is_low(),
            down: resources.BUTTONS.down.is_low(),
            left: resources.BUTTONS.left.is_low(),
            right: resources.BUTTONS.right.is_low(),
            center: resources.BUTTONS.center.is_low(),
        };

        resources.DISPLAY.draw(
            Circle::new(Coord::new(8, 2), 2)
                .with_stroke(Some(1u8.into()))
                .with_fill(Some((resources.BUTTON_STATE.up as u8).into()))
                .into_iter(),
        );

        resources.DISPLAY.draw(
            Circle::new(Coord::new(8, 14), 2)
                .with_stroke(Some(1u8.into()))
                .with_fill(Some((resources.BUTTON_STATE.down as u8).into()))
                .into_iter(),
        );

        resources.DISPLAY.draw(
            Circle::new(Coord::new(2, 8), 2)
                .with_stroke(Some(1u8.into()))
                .with_fill(Some((resources.BUTTON_STATE.left as u8).into()))
                .into_iter(),
        );

        resources.DISPLAY.draw(
            Circle::new(Coord::new(14, 8), 2)
                .with_stroke(Some(1u8.into()))
                .with_fill(Some((resources.BUTTON_STATE.right as u8).into()))
                .into_iter(),
        );

        resources.DISPLAY.draw(
            Circle::new(Coord::new(8, 8), 2)
                .with_stroke(Some(1u8.into()))
                .with_fill(Some((resources.BUTTON_STATE.center as u8).into()))
                .into_iter(),
        );

        resources.DISPLAY.flush().expect("Failed to update display");
    }
};
