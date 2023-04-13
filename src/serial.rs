/* New Tasks */

use bsp::hal;
use rp_pico as bsp;
use usb_device::UsbError;
use usbd_serial::SerialPort;
/// This function come from the github with USB-Serial example (see link above).
///
/// Helper function to ensure all data is written across the serial interface.
pub fn write_serial(serial: &mut SerialPort<'static, hal::usb::UsbBus>, buf: &str, block: bool) {
    let write_ptr = buf.as_bytes();

    // Because the buffer is of constant size and initialized to zero (0) we here
    // add a test to determine the size that's really occupied by the str that we
    // wan't to send. From index zero to first byte that is as the zero byte value.
    let mut index = 0;
    while index < write_ptr.len() && write_ptr[index] != 0 {
        index += 1;
    }
    let mut write_ptr = &write_ptr[0..index];

    while !write_ptr.is_empty() {
        match serial.write(write_ptr) {
            Ok(len) => write_ptr = &write_ptr[len..],
            // Meaning the USB write buffer is full
            Err(UsbError::WouldBlock) => {
                if !block {
                    break;
                }
            }
            // On error, just drop unwritten data.
            Err(_) => break,
        }
    }
    // let _ = serial.write("\n".as_bytes());
    let _ = serial.flush();
}
