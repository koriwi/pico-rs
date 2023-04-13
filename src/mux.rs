use super::hal;
use embedded_hal::digital::v2::OutputPin;

use alloc::vec::Vec;
use hal::gpio::DynPin;

// todo: read the pin range from sdcard
pub fn get_mux_pins(pin_array: &mut [Option<DynPin>; 29]) -> Vec<DynPin> {
    let mut mux_pins = Vec::with_capacity(3);
    for i in 20..23 {
        let mut pin = pin_array[i].take().unwrap();
        pin.into_push_pull_output();
        mux_pins.push(pin);
    }
    mux_pins
}

pub fn set_mux_addr(addr: u8, mux_pins: &mut [DynPin]) {
    for (index, pin) in mux_pins.iter_mut().enumerate() {
        match addr & (1 << index) {
            0 => pin.set_low().unwrap(),
            _ => pin.set_high().unwrap(),
        }
    }
}
