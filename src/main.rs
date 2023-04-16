#![no_std]
#![no_main]
mod button_machine;
mod misc;
mod mux;
mod overclock;

const BUTTON_COUNT: u8 = 8;
const ROW_SIZE: usize = 128;
const SD_MHZ: u32 = 12;
const I2C_KHZ: u32 = 1_200;

extern crate alloc;

use embedded_hal::spi::MODE_0;
use embedded_sdmmc::{Controller, Mode, VolumeIdx};
use misc::NineTeenSeventy;
use overclock::init_clocks_and_plls;
use rp_pico::hal::{
    self,
    gpio::{DynPin, FunctionI2C, Pins},
    pac,
    sio::Sio,
    timer::Timer,
    Clock, Spi,
};

use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use crate::mux::set_mux_addr;
use alloc_cortex_m::CortexMHeap;
use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use defmt::debug;
use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;

#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, 1024);
    }
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // The default is to generate a 125 MHz system clock
    let clocks = init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let sio = Sio::new(pac.SIO);

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut mux_pins: [Option<DynPin>; 4] = [
        Some(pins.gpio20.into()),
        Some(pins.gpio21.into()),
        Some(pins.gpio22.into()),
        None,
    ];

    for pin in mux_pins.iter_mut() {
        if pin.is_some() {
            pin.as_mut().unwrap().into_push_pull_output();
        }
    }

    let button_pin: DynPin = pins.gpio19.into_pull_up_input().into();
    let sda = pins.gpio2.into_mode::<FunctionI2C>();
    let scl = pins.gpio3.into_mode::<FunctionI2C>();

    // build i2c from dynpins
    let i2c = hal::i2c::I2C::new_controller(
        pac.I2C1,
        sda,
        scl,
        I2C_KHZ.kHz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    delay.delay_ms(1000);
    display.init().unwrap();
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);

    let _spi_sclk = pins.gpio10.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_mosi = pins.gpio11.into_mode::<hal::gpio::FunctionSpi>();
    let _spi_miso = pins.gpio8.into_mode::<hal::gpio::FunctionSpi>();
    let spi = Spi::<_, _, 8>::new(pac.SPI1);
    let spi = spi.init(
        &mut pac.RESETS,
        clocks.system_clock.freq(),
        SD_MHZ.MHz(),
        &MODE_0,
    );

    let sd_cs = pins.gpio9.into_push_pull_output();
    let mut spi_dev = embedded_sdmmc::SdMmcSpi::new(spi, sd_cs);
    let mut spi_dev: Controller<_, _, 128, 128> = match spi_dev.acquire() {
        Ok(block) => Controller::new(block, NineTeenSeventy {}),
        Err(e) => {
            debug!("{:?}", defmt::Debug2Format(&e));
            panic!("Failed to acquire SD card")
        }
    };
    let mut sdcard = spi_dev.get_volume(VolumeIdx(0)).unwrap();
    let root_dir = spi_dev.open_root_dir(&sdcard).unwrap();
    let mut config = spi_dev
        .open_file_in_dir(&mut sdcard, &root_dir, "config.bin", Mode::ReadOnly)
        .unwrap();

    let mut header = [0u8; ROW_SIZE];
    spi_dev.read(&sdcard, &mut config, &mut header).unwrap();

    let mut image = [0u8; 1024];
    for db in 0..(2_u8.pow(mux_pins.len() as u32)) {
        if db >= BUTTON_COUNT {
            break;
        }
        set_mux_addr(db, &mut mux_pins);
        delay.delay_ms(1); // wait for mux to settle
        display.init().unwrap();
        config
            .seek_from_start(header[2] as u32 * 128 + 1024 * db as u32)
            .unwrap();
        spi_dev.read(&sdcard, &mut config, &mut image).unwrap();
        display.draw(&image).unwrap();

        delay.delay_ms(1);
    }

    let mut callback = |action, index| {
        match action {
            button_machine::Actions::ShortDown => {
                if let Err(err) = display.draw(&image) {
                    debug!("{:?}", defmt::Debug2Format(&err));
                }
                display.draw(&image).unwrap();
            }
            button_machine::Actions::ShortUp => {
                display.clear();
                display.flush().unwrap();
            }
            _ => {}
        };
        debug!("action: {}: {:?}", index, action);
    };

    let mut button_machine =
        button_machine::ButtonMachine::new(&button_pin, 200, &timer, &mut callback);
    let mut button_index = 0;

    loop {
        if timer.get_counter().ticks() % 1000 == 0 {
            set_mux_addr(button_index, &mut mux_pins);
            delay.delay_us(10); // wait for mux to settle

            button_machine.check_button(button_index, false).unwrap();
            button_index += 1;
            if button_index > 7 {
                button_index = 0;
            }
        }

        // if serial.line_coding().data_rate() == 1200 {
        //     // Reset the board if the host sets the baud rate to 1200
        //     hal::rom_data::reset_to_usb_boot(0, 0);
        // }
        // usb_dev.poll(&mut [&mut serial]);
    }
}
