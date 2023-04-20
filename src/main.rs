#![no_std]
#![no_main]
mod button_machine;
mod config;
mod mux;
mod overclock;
mod sdcard;
mod util;

const BUTTON_COUNT: usize = 8;
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
use sdcard::SDConfigFile;
use sdcard::SpiPins;

use ssd1306::prelude::*;
use ssd1306::I2CDisplayInterface;
use ssd1306::Ssd1306;

use button_machine::*;
use config::*;

use crate::config::action::ButtonFunction;
use crate::mux::create_set_mux_addr;
use crate::util::retry;

use cortex_m_rt::entry;
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
    let mut set_mux_addr = create_set_mux_addr(mux_pins);

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

    let config_file = SDConfigFile::new(&mut sd_spi);

    let mut config = Config::new(config_file);
    config.load_page(0);

    for (i, button) in config.page.buttons.iter().enumerate() {
        set_mux_addr(i as u8);
        retry(|| display.init());
        retry(|| display.draw(button.image_buff()));
    }

    let mut button_changed = |event, index: u8| {
        let event = match event {
            Some(a) => a,
            None => {
                let max = BUTTON_COUNT as u8 - 1;
                let new_index = if index == max { 0 } else { index + 1 };
                set_mux_addr(new_index);
                return;
            }
        };
        let mut button = config.page.buttons[index as usize];

        let mut change_page = |target_page: u16| {
            config.load_page(target_page);
            for (i, button) in config.page.buttons.iter().enumerate() {
                set_mux_addr(i as u8);
                retry(|| display.draw(button.image_buff()));
            }
        };

        let key_down = |key: u8| {
            debug!("key_down: {}", key);
        };
        let key_up = |key: u8| {
            debug!("key_up: {}", key);
        };

        match event {
            ButtonEvent::ShortDown => match button.primary_function() {
                ButtonFunction::PressKeys(data) => {
                    for key in data.keys.iter() {
                        key_up(*key);
                    }
                }
                _ => {
                    debug!("short down");
                }
            },
            ButtonEvent::ShortUp => match button.primary_function() {
                ButtonFunction::ChangePage(data) => {
                    change_page(data.target_page);
                    debug!("change page: {}", data.target_page);
                }
                ButtonFunction::PressKeys(data) => {
                    for key in data.keys.iter() {
                        key_down(*key);
                    }
                }
                _ => {}
            },
            _ => {}
        };
        debug!("action: {}: {:?}", index, event);
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
