#![deny(unsafe_code)]
#![deny(warnings)]
#![feature(proc_macro)]
#![no_std]

extern crate cortex_m;
extern crate cortex_m_rtfm as rtfm;
extern crate panic_abort;
extern crate stm32f103xx_hal as hal;
extern crate embedded_graphics;
extern crate ssd1306;

use cortex_m::peripheral::syst::SystClkSource;
use rtfm::{app, Threshold};

use hal::prelude::*;
use hal::i2c::{DutyCycle, I2c, Mode};
use hal::stm32f103xx;
use hal::stm32f103xx::I2C1;
use hal::gpio::gpiob::{PB6, PB7};
use hal::gpio::{Alternate, OpenDrain};

use embedded_graphics::prelude::*;
use embedded_graphics::Drawing;
use embedded_graphics::fonts::{Font, Font6x8};
use ssd1306::Builder;
use ssd1306::mode::GraphicsMode;
use ssd1306::prelude::DisplaySize;
use ssd1306::interface::I2cInterface;

pub type OledDisplay =
    GraphicsMode<I2cInterface<I2c<I2C1, (PB6<Alternate<OpenDrain>>, PB7<Alternate<OpenDrain>>)>>>;

app! {
    device: stm32f103xx,

    resources: {
        static STATE: bool;
        static DISPLAY: OledDisplay;
    },

    tasks: {
        SYS_TICK: {
            path: sys_tick,
            resources: [STATE, DISPLAY],
        },
    },
}

fn init(p: init::Peripherals) -> init::LateResources {
    let mut flash = p.device.FLASH.constrain();
    let mut rcc = p.device.RCC.constrain();
    let mut afio = p.device.AFIO.constrain(&mut rcc.apb2);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);

    let mut syst = p.core.SYST;
    syst.set_clock_source(SystClkSource::Core);
    syst.set_reload(8_000_000);
    syst.enable_interrupt();
    syst.enable_counter();

    let mut gpiob = p.device.GPIOB.split(&mut rcc.apb2);

    let scl = gpiob.pb6.into_alternate_open_drain(&mut gpiob.crl);
    let sda = gpiob.pb7.into_alternate_open_drain(&mut gpiob.crl);
    let i2c1 = I2c::i2c1(
        p.device.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 200_000,
            duty_cycle: DutyCycle::Ratio1to1,
        },
        clocks,
        &mut rcc.apb1,
    );

    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_i2c(i2c1)
        .into();

    display.init().unwrap();
    display.clear();
    display.flush().unwrap();

    init::LateResources {
        STATE: false,
        DISPLAY: display,
    }
}

fn idle() -> ! {
    loop {
        rtfm::wfi();
    }
}

fn sys_tick(_t: &mut Threshold, mut r: SYS_TICK::Resources) {
    /*
    let state: &'static mut bool = r.STATE;
    let mut display: &'static mut OledDisplay = r.DISPLAY;

    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x32)
        .with_i2c_addr(0x3C)
        .connect_i2c(i2c1)
        .into();
    */

    r.DISPLAY.clear();

    match *r.STATE {
        true => r.DISPLAY.draw(Font6x8::render_str("TRUE").translate((0, 0)).into_iter()),
        false => r.DISPLAY.draw(Font6x8::render_str("FALSE").translate((0, 0)).into_iter()),
    }

    r.DISPLAY.flush().unwrap();
    *r.STATE = !*r.STATE;
}