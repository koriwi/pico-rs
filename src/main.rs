//! # Pico USB Serial Example
//!
//! Creates a USB Serial device on a Pico board, with the USB driver running in
//! the main thread.
//!
//! This will create a USB Serial device echoing anything it receives. Incoming
//! ASCII characters are converted to upercase, so you can tell it is working
//! and not just local-echo!
//!
//! See the `Cargo.toml` file for Copyright and license details.

#![no_std]
#![no_main]
extern crate alloc;
mod button_machine;
mod mux;
use defmt::{debug, println};
use rp_pico::{hal, Pins};

use alloc::boxed::Box;
// The macro for our start-up function
use cortex_m_rt::entry;
use defmt_rtt as _;
use hal::{gpio::DynPin, pac, Clock};

// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use alloc_cortex_m::CortexMHeap;
use panic_probe as _;

use crate::mux::{get_mux_pins, set_mux_addr};
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

fn create_pin_array(pins: Pins) -> [Option<DynPin>; 29] {
    let pin_array: [Option<DynPin>; 29] = [
        Some(pins.gpio0.into()),
        Some(pins.gpio1.into()),
        Some(pins.gpio2.into()),
        Some(pins.gpio3.into()),
        Some(pins.gpio4.into()),
        Some(pins.gpio5.into()),
        Some(pins.gpio6.into()),
        Some(pins.gpio7.into()),
        Some(pins.gpio8.into()),
        Some(pins.gpio9.into()),
        Some(pins.gpio10.into()),
        Some(pins.gpio11.into()),
        Some(pins.gpio12.into()),
        Some(pins.gpio13.into()),
        Some(pins.gpio14.into()),
        Some(pins.gpio15.into()),
        Some(pins.gpio16.into()),
        Some(pins.gpio17.into()),
        Some(pins.gpio18.into()),
        Some(pins.gpio19.into()),
        Some(pins.gpio20.into()),
        Some(pins.gpio21.into()),
        Some(pins.gpio22.into()),
        None,
        None,
        None,
        Some(pins.gpio26.into()),
        Some(pins.gpio27.into()),
        Some(pins.gpio28.into()),
    ];
    pin_array
}

/// Entry point to our bare-metal application.
///
/// The `#[entry]` macro ensures the Cortex-M start-up code calls this function
/// as soon as all global variables are initialised.
///
/// The function configures the RP2040 peripherals, then echoes any characters
/// received over USB Serial.
#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, 1024);
    }
    println!("free memory: {}", ALLOCATOR.free());
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
    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let sio = hal::Sio::new(pac.SIO);
    let pins = rp_pico::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut pin_array = create_pin_array(pins);

    // get pin programmatically by index
    let mut mux_pins = get_mux_pins(&mut pin_array);
    let mut button_pin = pin_array[19].take().unwrap();
    button_pin.into_pull_up_input();

    set_mux_addr(0, &mut mux_pins);

    let timer = hal::Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut button_index = 0;
    let mut button_machine = button_machine::ButtonMachine::new(
        &button_pin,
        200,
        &timer,
        Box::new(|action, index| {
            debug!("action: {}: {:?}", index, action);
        }),
    );
    println!("free memory: {}", ALLOCATOR.free());
    loop {
        if timer.get_counter().ticks() % 10000 == 0 {
            button_machine.check_button(button_index, true).unwrap();
            button_index += 1;
            if button_index > 7 {
                button_index = 0;
            }
            set_mux_addr(button_index, &mut mux_pins);
        }
        // if serial.line_coding().data_rate() == 1200 {
        //     // Reset the board if the host sets the baud rate to 1200
        //     hal::rom_data::reset_to_usb_boot(0, 0);
        // }
        // usb_dev.poll(&mut [&mut serial]);
    }
}
