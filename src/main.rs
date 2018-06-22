//! Blinks an LED

//#![deny(unsafe_code)]
//#![deny(warnings)]
#![no_std]
#![no_main]

#[macro_use]
extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32f103xx_hal as hal;
#[macro_use(block)]
extern crate nb;

use core::fmt::Write;

use core::ptr::read_volatile;
use core::ptr::write_volatile;
use cortex_m::asm;
use hal::prelude::*;
use hal::stm32f103xx;
use hal::timer::Timer;
use rt::ExceptionFrame;
use sh::hio;

entry!(main);

fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc1 = dp.RCC;
    let mut bkp = dp.BKP;
    let mut pwr = dp.PWR;

    let mut hstdout = hio::hstdout().unwrap();

    writeln!(hstdout, "apb1enr: {:#034b}", rcc1.apb1enr.read().bits()).unwrap();
    writeln!(hstdout, "cr: {:#034b}", pwr.cr.read().bits()).unwrap();

    // Enable backup registers
    rcc1.apb1enr.modify(|_,w| w.bkpen().enabled());
    rcc1.apb1enr.modify(|_,w| w.pwren().enabled());
    pwr.cr.modify(|_,w| w.dbp().bit(true));

    writeln!(hstdout, "apb1enr: {:#034b}", rcc1.apb1enr.read().bits()).unwrap();
    writeln!(hstdout, "cr: {:#034b}", pwr.cr.read().bits()).unwrap();

    let mut rcc = rcc1.constrain();

    // Try a different clock configuration
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    // let clocks = rcc.cfgr
    //     .sysclk(64.mhz())
    //     .pclk1(32.mhz())
    //     .freeze(&mut flash.acr);

    let mut gpioc = dp.GPIOC.split(&mut rcc.apb2);

    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    // Try a different timer (even SYST)
    let mut timer = Timer::syst(cp.SYST, 1.hz(), clocks);

    /*
    unsafe {
        write_volatile(0x40006C04 as *mut u32, 0x4F42u32);
        write_volatile(0x40006C08 as *mut u32, 0x544Fu32);
    }
    */
    bkp.dr1.write(|w| unsafe { w.bits(0x4F42u32) });
    bkp.dr2.write(|w| unsafe { w.bits(0x544Fu32) });

    /*
    let upper: u32 = unsafe { read_volatile(0x40006C04 as *mut u32) };
    let lower: u32 = unsafe { read_volatile(0x40006C08 as *mut u32) };
    */

    let upper: u32 = bkp.dr1.read().bits();
    let lower: u32 = bkp.dr2.read().bits();

    writeln!(hstdout, "upper: {:#X?}", upper).unwrap();
    writeln!(hstdout, "lower: {:#X?}", lower).unwrap();

    //iprintln!(&itm.stim[0], "upper: {}", upper);
    //iprintln!(&itm.stim[0], "lower: {}", lower);

    loop {
        block!(timer.wait()).unwrap();
        led.set_high();
        block!(timer.wait()).unwrap();
        led.set_low();
    }
}

exception!(HardFault, hard_fault);

fn hard_fault(ef: &ExceptionFrame) -> ! {
    panic!("{:#?}", ef);
}

exception!(*, default_handler);

fn default_handler(irqn: i16) {
    panic!("Unhandled exception (IRQn = {})", irqn);
}
