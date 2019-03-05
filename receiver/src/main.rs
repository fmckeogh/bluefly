#![no_std]
#![no_main]

extern crate panic_semihosting;

use cortex_m_semihosting::hprintln;
use rtfm::app;

const PERIOD: u32 = 1000;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    #[init]
    fn init() {
        hprintln!("init").unwrap();
        // bootstrap the `periodic` task
        //spawn.periodic().unwrap();
    }

    #[idle]
    fn idle() -> ! {
        loop {
            hprintln!("idle").unwrap();
        }
    }

    /*
    #[task(schedule = [periodic])]
    fn periodic() {
        hprintln!("periodic: scheduled:", scheduled).unwrap();

        schedule.periodic(scheduled + PERIOD.cycles()).unwrap();
    }
    */

    extern "C" {
        fn PDM();
    }
};
