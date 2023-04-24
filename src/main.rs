#![no_std]
#![no_main]
mod button_machine;
mod config;
mod functions;
mod mux;
mod overclock;
mod sdcard;
mod util;

const BUTTON_COUNT: usize = 8;
const SD_MHZ: u32 = 12;
const I2C_KHZ: u32 = 800;

use config::button::Button;
use cortex_m::delay::Delay;
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

use crate::config::action::ButtonFunction;
use crate::functions::Functions;
use crate::mux::create_set_mux_addr;
use crate::util::retry;

use cortex_m_rt::entry;
use defmt::Debug2Format;
use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;

#[entry]
fn main() -> ! {
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

    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let mut set_mux_addr = create_set_mux_addr(mux_pins, delay);

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

    debug!("tick");
    let config_file = SDConfigFile::new(&mut sd_spi);

    let mut config = config::Config::new(config_file);
    config.load_page(0);

    for (i, button) in config.page.buttons.iter().enumerate() {
        set_mux_addr(i as u8);
        retry(|| display.init());
        retry(|| display.draw(button.image_buff()));
    }
    debug!("tick");

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut button_machine = ButtonMachine::new(&button_pin, 200, &timer);
    let mut button_index = 0;

    loop {
        if timer.get_counter().ticks() % 1000 == 0 {
            let mut button2 = config.page.buttons[button_index];
            let mut button_changed = |event, button: &mut Button| {
                let mut functions = Functions::new(
                    &mut config,
                    &mut display,
                    &mut set_mux_addr,
                    &mut button_index,
                );
                let event = match event {
                    Some(a) => a,
                    None => {
                        functions.none();
                        return;
                    }
                };
                let function = match event {
                    ButtonEvent::ShortDown | ButtonEvent::ShortUp | ButtonEvent::ShortTriggered => {
                        button.primary_function()
                    }
                    ButtonEvent::LongTriggered => button.secondary_function(),
                };
                debug!("button changed: {:?} {:?}", event, Debug2Format(&function));
                match (function, event) {
                    (
                        ButtonFunction::ChangePage(data),
                        ButtonEvent::LongTriggered
                        | ButtonEvent::ShortTriggered
                        | ButtonEvent::ShortDown,
                    ) => {
                        functions.change_page(data.target_page);
                        set_mux_addr(button_index as u8);
                    }
                    _ => {}
                };
            };
            button_machine
                .check_button(&mut button2, &mut button_changed)
                .unwrap();
        }

        // if serial.line_coding().data_rate() == 1200 {
        //     // Reset the board if the host sets the baud rate to 1200
        //     hal::rom_data::reset_to_usb_boot(0, 0);
        // }
        // usb_dev.poll(&mut [&mut serial]);
    }
}
