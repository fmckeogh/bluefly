#![no_std]
#![no_main]

// We need to import this crate explicitly so we have a panic handler
extern crate panic_semihosting;

mod logger;

use {
    crate::logger::{BbqLogger, StampedLogger},
    bbqueue::{bbq, BBQueue, Consumer},
    core::fmt::Write,
    embedded_hal::adc::OneShot,
    log::{info, LevelFilter},
    nrf52810_hal::{
        self as hal,
        gpio::Level,
        nrf52810_pac::{self as pac, UARTE0},
        prelude::*,
        saadc::{Gain, Oversample, Reference, Resistor, Resolution, Saadc, SaadcConfig, Time},
        uarte::{Baudrate, Parity, Uarte},
    },
    rtfm::app,
    rubble::{
        beacon::Beacon,
        gatt::GattServer,
        l2cap::{BleChannelMap, L2CAPState},
        link::{
            ad_structure::AdStructure, queue, AddressKind, DeviceAddress, HardwareInterface,
            LinkLayer, Responder, MAX_PDU_SIZE,
        },
        security_manager::NoSecurity,
        time::Timer,
    },
    rubble_nrf52810::{
        radio::{BleRadio, PacketBuffer},
        timer::{BleTimer, StampSource},
    },
};

type Logger = StampedLogger<StampSource<pac::TIMER0>, BbqLogger>;

/// Hardware interface for the BLE stack (nRF52810 implementation).
pub struct HwNRf52810 {}

impl HardwareInterface for HwNRf52810 {
    type Timer = BleTimer<pac::TIMER0>;
    type Tx = BleRadio;
}

/// Stores the global logger used by the `log` crate.
static mut LOGGER: Option<logger::WriteLogger<Logger>> = None;

#[app(device = nrf52810_hal::nrf52810_pac)]
const APP: () = {
    static mut BLE_TX_BUF: PacketBuffer = [0; MAX_PDU_SIZE];
    static mut BLE_RX_BUF: PacketBuffer = [0; MAX_PDU_SIZE];
    static mut BLE_LL: LinkLayer<HwNRf52810> = ();
    static mut BLE_R: Responder<BleChannelMap<GattServer<'static>, NoSecurity>> = ();
    static mut RADIO: BleRadio = ();
    static mut BEACON_TIMER: pac::TIMER1 = ();
    static mut SERIAL: Uarte<UARTE0> = ();
    static mut LOG_SINK: Consumer = ();

    static mut ADC: Saadc = ();
    static mut ADC_PIN: hal::gpio::p0::P0_02<hal::gpio::Input<hal::gpio::Floating>> = ();

    #[init(resources = [BLE_TX_BUF, BLE_RX_BUF])]
    fn init() {
        //hprintln!("\n<< INIT >>\n").ok();

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

        {
            // Configure TIMER1 as the beacon timer. It's only used as a 16-bit timer.
            let timer = &mut device.TIMER1;
            timer.bitmode.write(|w| w.bitmode()._16bit());
            // prescaler = 2^9    = 512
            // 16 MHz / prescaler = 31_250 Hz
            timer.prescaler.write(|w| unsafe { w.prescaler().bits(9) }); // 0-9
            timer.intenset.write(|w| w.compare0().set());
            timer.shorts.write(|w| w.compare0_clear().enabled());
            timer.cc[0].write(|w| unsafe { w.bits(31_250 / 50) }); // ~50x per second
            timer.tasks_clear.write(|w| unsafe { w.bits(1) });

            timer.tasks_start.write(|w| unsafe { w.bits(1) });
        }

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

        let radio = BleRadio::new(device.RADIO, resources.BLE_TX_BUF, resources.BLE_RX_BUF);

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
        let (tx, _tx_cons) = queue::create(bbq![1024].unwrap());
        let (_rx_prod, rx) = queue::create(bbq![1024].unwrap());

        // Create the actual BLE stack objects
        let ll = LinkLayer::<HwNRf52810>::new(
            DeviceAddress::new([169, 255, 235, 206, 50, 121], AddressKind::Random),
            ble_timer,
        );

        let resp = Responder::new(
            tx,
            rx,
            L2CAPState::new(BleChannelMap::with_attributes(GattServer::new())),
        );

        let adc = {
            let config = SaadcConfig {
                resolution: Resolution::_14BIT,
                oversample: Oversample::OVER256X,
                reference: Reference::VDD1_4,
                gain: Gain::GAIN1_4,
                resistor: Resistor::BYPASS,
                time: Time::_40US,
            };

            Saadc::new(device.SAADC, config)
        };

        RADIO = radio;
        BLE_LL = ll;
        BLE_R = resp;
        BEACON_TIMER = device.TIMER1;
        SERIAL = serial;
        LOG_SINK = log_sink;

        ADC = adc;
        ADC_PIN = p0.p0_02.into_floating_input();
    }

    #[interrupt(resources = [RADIO, BLE_LL])]
    fn RADIO() {
        let next_update = resources
            .RADIO
            .recv_interrupt(resources.BLE_LL.timer().now(), &mut resources.BLE_LL);
        resources.BLE_LL.timer().configure_interrupt(next_update);
    }

    /// Fire the beacon.
    #[interrupt(resources = [BEACON_TIMER, RADIO, ADC, ADC_PIN])]
    fn TIMER1() {
        // acknowledge event
        resources.BEACON_TIMER.events_compare[0].reset();

        let device_address = DeviceAddress::new([169, 255, 235, 206, 50, 121], AddressKind::Random);

        let val: u16 = resources.ADC.read(resources.ADC_PIN).unwrap();

        info!("read val: {}", val);

        let beacon = Beacon::new(
            device_address,
            &[AdStructure::Unknown {
                ty: 0xFF,
                data: &[(val / 64) as u8],
            }],
        )
        .unwrap();

        beacon.broadcast(&mut *resources.RADIO);
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
