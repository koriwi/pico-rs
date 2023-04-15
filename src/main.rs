#![no_std]
#![no_main]
mod button_machine;
mod mux;

extern crate alloc;

use rp_pico::hal::{
    self,
    gpio::{DynPin, FunctionI2C, Pins},
    pac,
    sio::Sio,
    timer::Timer,
    Clock,
};

use embedded_graphics::{
    image::ImageRaw,
    pixelcolor::{
        self,
        raw::{ByteOrder, LittleEndian},
        BinaryColor,
    },
    prelude::*,
    primitives::{Circle, PrimitiveStyleBuilder, Rectangle, Triangle},
};
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};

use alloc_cortex_m::CortexMHeap;
use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use defmt::debug;
use defmt_rtt as _;
use fugit::RateExtU32;
use panic_probe as _;

use crate::mux::set_mux_addr;
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, 1024);
    }
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
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
        800u32.kHz(),
        &mut pac.RESETS,
        clocks.system_clock.freq(),
    );
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    for db in 0..(2_u8.pow(mux_pins.len() as u32)) {
        set_mux_addr(db, &mut mux_pins);
        delay.delay_us(10); // wait for mux to settle
        display.init().unwrap();
    }
    display.init().unwrap();
    // let raw: ImageRaw<'static, pixelcolor::raw::LittleEndian> =
    //     ImageRaw::new(include_bytes!("./kilian.raw"), 128);
    let image = include_bytes!("./back.raw");
    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut callback = |action, index| {
        match action {
            button_machine::Actions::ShortDown => {
                match display.draw(image) {
                    Ok(_) => {}
                    Err(_) => debug!("error"),
                };
                display.flush().unwrap();
            }
            button_machine::Actions::LongTriggered => {
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

            button_machine.check_button(button_index, true).unwrap();
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
