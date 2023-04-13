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
use alloc::boxed::Box;
use defmt::{debug, println};
use embedded_hal::digital::v2::{InputPin, OutputPin};
// The macro for our start-up function
use defmt_rtt as _;
use fugit::{Instant, RateExtU32};
use rp_pico::{
    entry,
    hal::{
        gpio::{DynInput, DynPin, DynPinId, DynPinMode},
        uart::{DataBits, StopBits, UartConfig},
        Clock,
    },
    Pins,
};
// Ensure we halt the program on panic (if we don't mention this crate it won't
// be linked)
use panic_halt as _;

// A shorter alias for the Peripheral Access Crate, which provides low-level
// register access
use rp_pico::hal::pac;

// A shorter alias for the Hardware Abstraction Layer, which provides
// higher-level drivers.
use rp_pico::hal;

// USB Device support
use usb_device::{class_prelude::*, prelude::*};

// USB Communications Class Device support
use usbd_serial::SerialPort;

// Used to demonstrate writing formatted strings
use alloc_cortex_m::CortexMHeap;
use core::fmt::Write;
use heapless::String;
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

fn get_mux_addr_bits(addr: u8) -> (bool, bool, bool) {
    let mut addr_bits = (false, false, false);
    if addr & 0b0001 != 0 {
        addr_bits.0 = true;
    }
    if addr & 0b0010 != 0 {
        addr_bits.1 = true;
    }
    if addr & 0b0100 != 0 {
        addr_bits.2 = true;
    }
    addr_bits
}

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
    // Grab our singleton objects
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // Configure the clocks
    //
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

    // Set up the USB driver
    // let usb_bus = UsbBusAllocator::new(hal::usb::UsbBus::new(
    //     pac.USBCTRL_REGS,
    //     pac.USBCTRL_DPRAM,
    //     clocks.usb_clock,
    //     true,
    //     &mut pac.RESETS,
    // ));

    // Set up the USB Communications Class Device driver
    // let mut serial = SerialPort::new(&usb_bus);
    // Create a USB device with a fake VID and PID
    // let mut usb_dev = UsbDeviceBuilder::new(&usb_bus, UsbVidPid(0x16c0, 0x27dd))
    //     .manufacturer("Fake company")
    //     .product("Serial port")
    //     .serial_number("TEST")
    //     .device_class(2) // from: https://www.usb.org/defined-class-codes
    //     .build();

    // get pin programmatically by index
    let mut mux_s0 = pin_array[20].take().unwrap();
    let mut mux_s1 = pin_array[21].take().unwrap();
    let mut mux_s2 = pin_array[22].take().unwrap();
    let mut button_pin = pin_array[19].take().unwrap();

    mux_s0.into_push_pull_output();
    mux_s1.into_push_pull_output();
    mux_s2.into_push_pull_output();
    button_pin.into_pull_up_input();

    let bits = get_mux_addr_bits(0);
    if bits.0 {
        mux_s0.set_high().unwrap();
    } else {
        mux_s0.set_low().unwrap();
    }
    if bits.1 {
        mux_s1.set_high().unwrap();
    } else {
        mux_s1.set_low().unwrap();
    }
    if bits.2 {
        mux_s2.set_high().unwrap();
    } else {
        mux_s2.set_low().unwrap();
    }

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

    loop {
        if timer.get_counter().ticks() % 10000 == 0 {
            button_machine.check_button(button_index, true).unwrap();
            button_index += 1;
            if button_index > 7 {
                button_index = 0;
            }
            let bits = get_mux_addr_bits(button_index);
            if bits.0 {
                mux_s0.set_high().unwrap();
            } else {
                mux_s0.set_low().unwrap();
            }
            if bits.1 {
                mux_s1.set_high().unwrap();
            } else {
                mux_s1.set_low().unwrap();
            }
            if bits.2 {
                mux_s2.set_high().unwrap();
            } else {
                mux_s2.set_low().unwrap();
            }
        }
        // if serial.line_coding().data_rate() == 1200 {
        //     // Reset the board if the host sets the baud rate to 1200
        //     hal::rom_data::reset_to_usb_boot(0, 0);
        // }
        // usb_dev.poll(&mut [&mut serial]);

        // // A welcome message at the beginning
        // if !said_hello && timer.get_counter().ticks() >= 2_000_000 {
        //     said_hello = true;
        //     let _ = serial.write(b"Hello, World!\r\n");

        //     let time = timer.get_counter().ticks();
        //     let mut text: String<64> = String::new();
        //     writeln!(&mut text, "Current timer ticks: {}", time).unwrap();

        //     // This only works reliably because the number of bytes written to
        //     // the serial port is smaller than the buffers available to the USB
        //     // peripheral. In general, the return value should be handled, so that
        //     // bytes not transferred yet don't get lost.
        //     let _ = serial.write(text.as_bytes());
        // }

        // // Check for new data
        // if usb_dev.poll(&mut [&mut serial]) {
        //     let mut buf = [0u8; 64];
        //     match serial.read(&mut buf) {
        //         Err(_e) => {
        //             // Do nothing
        //         }
        //         Ok(0) => {
        //             // Do nothing
        //         }
        //         Ok(count) => {
        //             // Convert to upper case
        //             buf.iter_mut().take(count).for_each(|b| {
        //                 b.make_ascii_uppercase();
        //             });
        //             // Send back to the host
        //             let mut wr_ptr = &buf[..count];
        //             while !wr_ptr.is_empty() {
        //                 match serial.write(wr_ptr) {
        //                     Ok(len) => wr_ptr = &wr_ptr[len..],
        //                     // On error, just drop unwritten data.
        //                     // One possible error is Err(WouldBlock), meaning the USB
        //                     // write buffer is full.
        //                     Err(_) => break,
        //                 };
        //             }
        //         }
        //     }
        // }
    }
}
