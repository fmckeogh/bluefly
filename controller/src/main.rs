#![no_std]
#![no_main]

// We need to import this crate explicitly so we have a panic handler
extern crate panic_semihosting;

pub mod ble;
mod radio;

use {
    crate::{
        ble::{
            beacon::Beacon,
            link::{
                ad_structure::AdStructure, AddressKind, DeviceAddress, LinkLayer, MAX_PDU_SIZE,
            },
        },
        radio::{Baseband, BleRadio, PacketBuffer},
    },
    byteorder::{ByteOrder, LittleEndian},
    core::{fmt::Write, time::Duration, u32},
    nrf52810_hal::{
        self as hal,
        gpio::Level,
        nrf52810_pac::{self as pac, UARTE0},
        prelude::*,
        uarte::{Baudrate, Parity, Uarte},
    },
    rtfm::app,
};

type Logger = Uarte<UARTE0>;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut BLE_TX_BUF: PacketBuffer = [0; MAX_PDU_SIZE + 1];
    static mut BLE_RX_BUF: PacketBuffer = [0; MAX_PDU_SIZE + 1];
    static mut BASEBAND: Baseband<Logger> = ();
    static mut BEACON_TIMER: pac::TIMER1 = ();
    static BLE_TIMER: pac::TIMER0 = ();
    static mut ADC: pac::SAADC = ();
    static ADDR: DeviceAddress = ();

    #[init(resources = [BLE_TX_BUF, BLE_RX_BUF])]
    fn init() {
        let p0 = device.P0.split();

        let mut serial = {
            let rxd = p0.p0_08.into_floating_input().degrade();
            let txd = p0.p0_31.into_push_pull_output(Level::Low).degrade();

            let pins = hal::uarte::Pins {
                rxd,
                txd,
                cts: None,
                rts: None,
            };

            device
                .UARTE0
                .constrain(pins, Parity::EXCLUDED, Baudrate::BAUD1M)
        };
        writeln!(serial, "\n--- INIT ---").unwrap();

        {
            // On reset the internal high frequency clock is used, but starting the HFCLK task
            // switches to the external crystal; this is needed for Bluetooth to work.

            device
                .CLOCK
                .tasks_hfclkstart
                .write(|w| unsafe { w.bits(1) });
            while device.CLOCK.events_hfclkstarted.read().bits() == 0 {}
        }

        {
            // Configure TIMER1 as the beacon timer. It's only a 16-bit timer.
            let timer = &mut device.TIMER1;
            timer.bitmode.write(|w| w.bitmode()._16bit());
            // prescaler = 2^9    = 512
            // 16 MHz / prescaler = 31_250 Hz
            timer.prescaler.write(|w| unsafe { w.prescaler().bits(9) }); // 0-9
            timer.intenset.write(|w| w.compare0().set());
            timer.shorts.write(|w| w.compare0_clear().enabled());
            timer.cc[0].write(|w| unsafe { w.bits(31_250 / 50) }); // 50 times a second
            timer.tasks_clear.write(|w| unsafe { w.bits(1) });

            timer.tasks_start.write(|w| unsafe { w.bits(1) });
        }

        {
            // Configure ADC
            device.SAADC.enable.write(|w| w.enable().enabled());
            device.SAADC.resolution.write(|w| w.val()._12bit());
            device.SAADC.oversample.write(|w| w.oversample().bypass());
            device.SAADC.samplerate.write(|w| w.mode().task());

            device.SAADC.ch[0].config.write(|w| {
                w.refsel()
                    .internal()
                    .gain()
                    .gain1_6()
                    .tacq()
                    ._10us()
                    .mode()
                    .se()
                    .resp()
                    .bypass()
                    .resn()
                    .bypass()
                    .burst()
                    .disabled()
            });
            device.SAADC.ch[0]
                .pselp
                .write(|w| w.pselp().analog_input0());
            device.SAADC.ch[0].pseln.write(|w| w.pseln().nc());

            // Calibrate
            device
                .SAADC
                .tasks_calibrateoffset
                .write(|w| w.tasks_calibrateoffset().trigger());
            writeln!(serial, "calibrating adc").unwrap();
            while device
                .SAADC
                .events_calibratedone
                .read()
                .events_calibratedone()
                .is_not_generated()
            {}

            device.SAADC.inten.write(|w| w.end().enabled());
        }

        let device_address = {
            let mut devaddr = [0u8; 6];
            let devaddr_lo = device.FICR.deviceaddr[0].read().bits();
            let devaddr_hi = device.FICR.deviceaddr[1].read().bits() as u16;
            LittleEndian::write_u32(&mut devaddr, devaddr_lo);
            LittleEndian::write_u16(&mut devaddr[4..], devaddr_hi);

            writeln!(serial, "devaddr: {:?}", devaddr).unwrap();

            let devaddr_type = if device
                .FICR
                .deviceaddrtype
                .read()
                .deviceaddrtype()
                .is_public()
            {
                AddressKind::Public
            } else {
                AddressKind::Random
            };
            DeviceAddress::new(devaddr, devaddr_type)
        };

        let ll = LinkLayer::with_logger(device_address, serial);

        let baseband = Baseband::new(
            BleRadio::new(device.RADIO, resources.BLE_TX_BUF),
            resources.BLE_RX_BUF,
            ll,
        );

        // Queue first baseband update
        cfg_timer(&device.TIMER0, Some(Duration::from_millis(1)));

        BEACON_TIMER = device.TIMER1;
        BASEBAND = baseband;
        BLE_TIMER = device.TIMER0;
        ADDR = device_address;
        ADC = device.SAADC;
    }

    #[interrupt(resources = [BLE_TIMER, BASEBAND])]
    fn RADIO() {
        if let Some(new_timeout) = resources.BASEBAND.interrupt() {
            cfg_timer(&resources.BLE_TIMER, Some(new_timeout));
        }
    }

    /// Fire the beacon.
    #[interrupt(resources = [BEACON_TIMER, BASEBAND, ADDR, ADC])]
    fn TIMER1() {
        // acknowledge event
        resources.BEACON_TIMER.events_compare[0].reset();

        let log = resources.BASEBAND.logger();
        writeln!(log, "-> beacon").unwrap();

        // Read ADC
        let buf = [0u8; 2];

        resources
            .ADC
            .result
            .ptr
            .write(|w| unsafe { w.ptr().bits(buf.as_ptr() as u32) });
        resources
            .ADC
            .result
            .maxcnt
            .write(|w| unsafe { w.maxcnt().bits(2) });

        resources
            .ADC
            .tasks_sample
            .write(|w| w.tasks_sample().trigger());

        while resources
            .ADC
            .events_done
            .read()
            .events_done()
            .is_not_generated()
        {
            writeln!(log, "waiting for adc DONE event").unwrap();
        }

        writeln!(
            log,
            "amount: {}",
            resources.ADC.result.amount.read().amount().bits()
        )
        .unwrap();
        writeln!(log, "adc: {:?}", buf).unwrap();

        let beacon = Beacon::new(
            *resources.ADDR,
            &[AdStructure::Unknown {
                ty: 0xFF,
                data: &[120u8],
            }],
        )
        .unwrap();
        beacon.broadcast(resources.BASEBAND.transmitter());
    }

    #[interrupt(resources = [ADC])]
    fn SAADC() {
        panic!(
            "saadc end interrupt, amount: {}",
            resources.ADC.result.amount.read().bits()
        );
    }
};

/// Reconfigures TIMER0 to raise an interrupt after `duration` has elapsed.
///
/// TIMER0 is stopped if `duration` is `None`.
///
/// Note that if the timer has already queued an interrupt, the task will still be run after the
/// timer is stopped by this function.
fn cfg_timer(t: &pac::TIMER0, duration: Option<Duration>) {
    // Timer activation code is also copied from the `nrf51-hal` crate.
    if let Some(duration) = duration {
        assert!(duration.as_secs() < ((u32::MAX - duration.subsec_micros()) / 1_000_000) as u64);
        let us = (duration.as_secs() as u32) * 1_000_000 + duration.subsec_micros();
        t.cc[0].write(|w| unsafe { w.bits(us) });
        // acknowledge last compare event (FIXME unnecessary?)
        t.events_compare[0].reset();
        t.tasks_clear.write(|w| unsafe { w.bits(1) });
        t.tasks_start.write(|w| unsafe { w.bits(1) });
    } else {
        t.tasks_stop.write(|w| unsafe { w.bits(1) });
        t.tasks_clear.write(|w| unsafe { w.bits(1) });
        // acknowledge last compare event (FIXME unnecessary?)
        t.events_compare[0].reset();
    }
}
