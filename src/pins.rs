use super::hal::gpio::{DynPin, Pins};

pub fn create_pin_array(pins: Pins) -> [Option<DynPin>; 29] {
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
