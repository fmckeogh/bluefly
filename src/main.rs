#![deny(unsafe_code)]
#![deny(warnings)]
#![no_std]

extern crate cortex_m;
extern crate panic_abort;
extern crate stm32f103xx_hal as hal;
extern crate embedded_graphics;
extern crate ssd1306;

use hal::prelude::*;
use hal::i2c::{DutyCycle, I2c, Mode};
use hal::stm32f103xx;

use embedded_graphics::prelude::*;
use embedded_graphics::Drawing;
use embedded_graphics::fonts::{Font, Font6x8};
use ssd1306::{mode::GraphicsMode, Builder, DisplaySize};


fn main() {
    let _cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);

    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    //let mut timer = Timer::syst(cp.SYST, 1.hz(), clocks);


    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);
//    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);

//    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);

    let scl = gpiob.pb6.into_alternate_open_drain(&mut gpiob.crl);
    let sda = gpiob.pb7.into_alternate_open_drain(&mut gpiob.crl);

    let i2c = I2c::i2c1(
        dp.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 200_000,
            duty_cycle: DutyCycle::Ratio1to1,
        },
        clocks,
        &mut rcc.apb1,
    );

    let mut disp: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x64)
        .connect_i2c(i2c)
        .into();

    disp.init().unwrap();
    disp.flush().unwrap();

    disp.draw(Font6x8::render_str("Hello, world!").translate((0, 0)).into_iter());

    disp.flush().unwrap();
}
/*
// Called to nearly continuously read data from input sensors and update values in memory
fn read_input () {
}
// Called on packet received interrupt from CC1101
fn read_packet() {
    // Decrypt in GCM
    // Write to screen?
    // Depends how long that takes, I don't want to miss a packet, nor do I want to have to build a buffer system
}
// Called every n milliseconds, sends currently stored values in memory
fn send_packet() {
}
*/