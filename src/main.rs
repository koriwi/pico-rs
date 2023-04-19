#![no_std]
#![no_main]
mod button;
mod button_machine;
mod config;
mod macros;
mod misc;
mod mux;
mod overclock;
mod sdcard;
mod short;

const BUTTON_COUNT: usize = 8;
const ROW_SIZE: usize = 128;
const SD_MHZ: u32 = 12;
const I2C_KHZ: u32 = 800;

use overclock::init_clocks_and_plls;

use rp_pico::hal;
use rp_pico::hal::gpio::DynPin;
use rp_pico::hal::gpio::FunctionI2C;
use rp_pico::hal::gpio::Pins;
use rp_pico::hal::pac;
use rp_pico::hal::sio::Sio;
use rp_pico::hal::timer::Timer;
use rp_pico::hal::Clock;

use sdcard::create_sdcard;
use sdcard::SpiPins;

use ssd1306::prelude::*;
use ssd1306::I2CDisplayInterface;
use ssd1306::Ssd1306;

use crate::button::ButtonFunction;
use crate::button_machine::Actions;
use crate::button_machine::ButtonMachine;
use crate::config::Config;
use crate::misc::retry;
use crate::mux::set_mux_addr;

use cortex_m_rt::entry;
use defmt::debug;
use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;

#[entry]
fn main() -> ! {
    let mut pac = pac::Peripherals::take().unwrap();
    // let core = pac::CorePeripherals::take().unwrap();
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

    // let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
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

    let spi_pins = SpiPins::new(
        pins.gpio10.into(),
        pins.gpio11.into(),
        pins.gpio8.into(),
        pins.gpio9.into(),
    );

    let mut sd_spi = create_sdcard(
        pac.SPI1,
        spi_pins,
        clocks.system_clock.freq(),
        SD_MHZ,
        &mut pac.RESETS,
    );

    let mut config = Config::new(&mut sd_spi);
    config.read_page_data(0);

    for (i, button) in config.buttons.iter().enumerate() {
        set_mux_addr(i as u8, &mut mux_pins);
        retry(|| display.init());
        retry(|| display.draw(button.get_image()));
    }

    let mut button_changed = |action, index: u8| {
        let action = match action {
            Some(a) => a,
            None => {
                let max = BUTTON_COUNT as u8 - 1;
                let new_index = if index == max { 0 } else { index + 1 };
                set_mux_addr(new_index, &mut mux_pins);
                return;
            }
        };

        let primary = config.get_primary_function(index as usize);
        // let (secondary_function, secondary_data) = config.get_secondary_function(index as usize);

        match action {
            Actions::ShortDown => {
                debug!("short down");
            }
            Actions::ShortUp => match primary.function {
                ButtonFunction::ChangePage => {
                    config
                        .read_page_data(u16::from_le_bytes(primary.data[0..2].try_into().unwrap()));
                    config.buttons.iter().enumerate().for_each(|(i, b)| {
                        set_mux_addr(i as u8, &mut mux_pins);
                        retry(|| display.draw(b.get_image()));
                    });
                }
                _ => {
                    debug!("short up");
                }
            },
            _ => {}
        };
        debug!("action: {}: {:?}", index, action);
    };

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut button_machine = ButtonMachine::new(&button_pin, 200, &timer, &mut button_changed);
    let mut button_index = 0;

    loop {
        if timer.get_counter().ticks() % 1000 == 0 {
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
