use super::hal;
use cortex_m::delay::Delay;
use embedded_hal::digital::v2::OutputPin;

use hal::gpio::DynPin;

pub fn create_set_mux_addr(mut mux_pins: [Option<DynPin>; 4], mut delay: Delay) -> impl FnMut(u8) {
    move |addr| {
        for (index, pin) in mux_pins.iter_mut().enumerate() {
            if pin.is_none() {
                return;
            }
            match addr & (1 << index) {
                0 => pin.as_mut().unwrap().set_low().unwrap(),
                _ => pin.as_mut().unwrap().set_high().unwrap(),
            }
        }
        delay.delay_ms(3_u32);
    }
}
