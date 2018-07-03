#![no_std]
#![no_main]

extern crate cortex_m;
#[macro_use]
extern crate cortex_m_rt as rt;
extern crate cortex_m_semihosting as sh;
extern crate panic_semihosting;
extern crate stm32f103xx_hal as hal;
#[macro_use(block)]
extern crate nb;

use hal::prelude::*;
use hal::stm32f103xx;
use hal::timer::Timer;
use rt::ExceptionFrame;

entry!(main);

fn main() -> ! {
    let cp = cortex_m::Peripherals::take().unwrap();
    let dp = stm32f103xx::Peripherals::take().unwrap();

    let mut flash = dp.FLASH.constrain();
    let mut rcc = dp.RCC;
    let mut bkp = dp.BKP;
    let mut pwr = dp.PWR;

    // Enable backup registers
    rcc.apb1enr.modify(|_, w| w.bkpen().enabled());
    rcc.apb1enr.modify(|_, w| w.pwren().enabled());
    pwr.cr.modify(|_, w| w.dbp().bit(true));

    let mut rcc_c = rcc.constrain();

    let mut gpioc = dp.GPIOC.split(&mut rcc_c.apb2);
    let clocks = rcc_c.cfgr.freeze(&mut flash.acr);

    let mut led = gpioc.pc13.into_push_pull_output(&mut gpioc.crh);
    let mut timer = Timer::syst(cp.SYST, 1.hz(), clocks);

    let mut count: u64 = 0;

    loop {
        if count > 4 {
            bkp.dr1.write(|w| unsafe { w.bits(0x4F42u32) });
            bkp.dr2.write(|w| unsafe { w.bits(0x544Fu32) });
            led.set_low();
            while true {}
        }
        count += 1;

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
