#![no_std]
#![no_main]

// We need to import this crate explicitly so we have a panic handler
extern crate panic_semihosting;

mod logger;
mod radio;
mod timer;

use {
    crate::logger::{BbqLogger, StampedLogger},
    crate::{
        radio::{BleRadio, PacketBuffer},
        timer::{BleTimer, StampSource},
    },
    bbqueue::{bbq, BBQueue, Consumer},
    core::fmt::Write,
    log::{info, LevelFilter},
    nrf52810_hal::{
        self as hal,
        gpio::Level,
        nrf52810_pac::{self as pac, UARTE0},
        prelude::*,
        uarte::{Baudrate, Parity, Uarte},
    },
    rtfm::app,
    rubble::{
        beacon::{BeaconScanner, ScanCallback},
        gatt::GattServer,
        l2cap::{BleChannelMap, L2CAPState},
        link::{
            ad_structure::AdStructure, filter::WhitelistFilter, queue, AddressKind, DeviceAddress,
            HardwareInterface, LinkLayer, Responder, MAX_PDU_SIZE,
        },
        security_manager::NoSecurity,
        time::{Duration, Timer},
    },
};

type Logger = StampedLogger<StampSource<pac::TIMER0>, BbqLogger>;

/// Hardware interface for the BLE stack (nRF52810 implementation).
pub struct HwNRf52810 {}

impl HardwareInterface for HwNRf52810 {
    type Timer = BleTimer<pac::TIMER0>;
    type Tx = BleRadio;
}

/// Whether to broadcast a beacon or to establish a proper connection.
///
/// This is just used to test different code paths. Note that you can't do both
/// at the same time unless you also generate separate device addresses.
const TEST_BEACON: bool = false;

/// Stores the global logger used by the `log` crate.
static mut LOGGER: Option<logger::WriteLogger<Logger>> = None;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut BLE_TX_BUF: PacketBuffer = [0; MAX_PDU_SIZE];
    static mut BLE_RX_BUF: PacketBuffer = [0; MAX_PDU_SIZE];
    static mut BLE_LL: LinkLayer<HwNRf52810> = ();
    static mut BLE_R: Responder<BleChannelMap<GattServer<'static>, NoSecurity>> = ();
    static mut RADIO: BleRadio = ();
    static mut SCANNER: rubble::beacon::BeaconScanner<
        PwmCallback,
        rubble::link::filter::WhitelistFilter<core::iter::Once<rubble::link::DeviceAddress>>,
    > = ();
    static mut SERIAL: Uarte<UARTE0> = ();
    static mut LOG_SINK: Consumer = ();

    #[init(resources = [BLE_TX_BUF, BLE_RX_BUF])]
    fn init() {
        {
            // On reset the internal high frequency clock is used, but starting the HFCLK task
            // switches to the external crystal; this is needed for Bluetooth to work.

            device
                .CLOCK
                .tasks_hfclkstart
                .write(|w| unsafe { w.bits(1) });
            while device.CLOCK.events_hfclkstarted.read().bits() == 0 {}
        }

        let ble_timer = BleTimer::init(device.TIMER0);

        let p0 = device.P0.split();

        let mut serial = {
            let rxd = p0.p0_08.into_floating_input().degrade();
            let txd = p0.p0_06.into_push_pull_output(Level::Low).degrade();

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

        let device_address = DeviceAddress::new([0, 0, 0, 0, 0, 0], AddressKind::Random);

        let mut radio = BleRadio::new(device.RADIO, resources.BLE_TX_BUF, resources.BLE_RX_BUF);

        let log_stamper = ble_timer.create_stamp_source();
        let (tx, log_sink) = bbq![10000].unwrap().split();
        let logger = StampedLogger::new(BbqLogger::new(tx), log_stamper);

        let log = logger::WriteLogger::new(logger);
        // Safe, since we're the only thread and interrupts are off
        unsafe {
            LOGGER = Some(log);
            log::set_logger(LOGGER.as_ref().unwrap()).unwrap();
        }
        log::set_max_level(LevelFilter::max());

        info!("READY");

        // Create TX/RX queues
        let (tx, tx_cons) = queue::create(bbq![1024].unwrap());
        let (rx_prod, rx) = queue::create(bbq![1024].unwrap());

        // Create the actual BLE stack objects
        let mut ll = LinkLayer::<HwNRf52810>::new(device_address, ble_timer);

        let resp = Responder::new(
            tx,
            rx,
            L2CAPState::new(BleChannelMap::with_attributes(GattServer::new())),
        );

        if !TEST_BEACON {
            // Send advertisement and set up regular interrupt
            let next_update = ll
                .start_advertise(
                    Duration::from_millis(200),
                    &[AdStructure::CompleteLocalName("CONCVRRENS CERTA CELERIS")],
                    &mut radio,
                    tx_cons,
                    rx_prod,
                )
                .unwrap();
            ll.timer().configure_interrupt(next_update);
        }

        let scanner = {
            let filter = WhitelistFilter::from_address(DeviceAddress::new(
                [169, 255, 235, 206, 50, 121],
                AddressKind::Random,
            ));

            let callback = {
                device.PWM0.psel.out[0]
                    .write(|w| unsafe { w.pin().bits(0x08).connect().connected() });
                device.PWM0.enable.write(|w| w.enable().enabled());
                device.PWM0.mode.write(|w| w.updown().up());
                device.PWM0.prescaler.write(|w| w.prescaler().div_32());
                device
                    .PWM0
                    .countertop
                    .write(|w| unsafe { w.countertop().bits(8_000) }); // 20ms at div_32
                device.PWM0.loop_.write(|w| w.cnt().disabled());
                device
                    .PWM0
                    .decoder
                    .write(|w| w.load().common().mode().next_step());
                device.PWM0.seq0.refresh.write(|w| w.cnt().continuous());
                device
                    .PWM0
                    .seq0
                    .enddelay
                    .write(|w| unsafe { w.cnt().bits(0) });
                let mut val: u16 = 7220;
                device.PWM0.seq0.cnt.write(|w| unsafe { w.cnt().bits(1) });
                device
                    .PWM0
                    .seq0
                    .ptr
                    .write(|w| unsafe { w.ptr().bits(((&mut val) as *mut _) as u32) });

                device.PWM0.tasks_seqstart[0].write(|w| w.tasks_seqstart().trigger());
                device
                    .PWM0
                    .tasks_nextstep
                    .write(|w| w.tasks_nextstep().trigger());

                PwmCallback(device.PWM0)
            };

            BeaconScanner::with_filter(callback, filter)
        };

        RADIO = radio;
        BLE_LL = ll;
        BLE_R = resp;
        SCANNER = scanner;
        SERIAL = serial;
        LOG_SINK = log_sink;
    }

    #[interrupt(resources = [RADIO, BLE_LL, SCANNER])]
    fn RADIO() {
        let next_update = resources
            .RADIO
            .recv_interrupt(resources.BLE_LL.timer().now(), &mut resources.SCANNER);

        //let cmd = resources.SCANNER.process_adv_packet()
        resources.BLE_LL.timer().configure_interrupt(next_update);
    }

    #[interrupt(resources = [RADIO, BLE_LL, SCANNER])]
    fn TIMER0() {
        let timer = resources.BLE_LL.timer();
        if !timer.is_interrupt_pending() {
            return;
        }
        timer.clear_interrupt();

        let cmd = resources.BLE_LL.update(&mut *resources.RADIO);
        resources.RADIO.configure_receiver(cmd.radio);

        resources
            .BLE_LL
            .timer()
            .configure_interrupt(cmd.next_update);
    }

    #[idle(resources = [LOG_SINK, SERIAL, BLE_R])]
    fn idle() -> ! {
        // Drain the logging buffer through the serial connection
        loop {
            while let Ok(grant) = resources.LOG_SINK.read() {
                for chunk in grant.buf().chunks(255) {
                    resources.SERIAL.write(chunk).unwrap();
                }

                resources.LOG_SINK.release(grant.buf().len(), grant);
            }

            if resources.BLE_R.has_work() {
                resources.BLE_R.process_one().unwrap();
            }
        }
    }
};

pub struct PwmCallback(pac::PWM0);

impl ScanCallback for PwmCallback {
    fn beacon<'a, I>(&mut self, _adv_addr: DeviceAddress, adv_data: I)
    where
        I: Iterator<Item = AdStructure<'a>>,
    {
        match adv_data.last() {
            Some(AdStructure::Unknown { ty: _, data }) => {
                info!("got val: {}", data[0]);

                let mut val: u16 = 6990 + (u16::from(data[0]) * 2);
                self.0.seq0.cnt.write(|w| unsafe { w.cnt().bits(1) });
                self.0
                    .seq0
                    .ptr
                    .write(|w| unsafe { w.ptr().bits(((&mut val) as *mut _) as u32) });

                self.0.tasks_seqstart[0].write(|w| w.tasks_seqstart().trigger());
                self.0
                    .tasks_nextstep
                    .write(|w| w.tasks_nextstep().trigger());
            }
            _ => (),
        }
    }
}
