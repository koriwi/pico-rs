use defmt::debug;

use crate::{button::ButtonFunction, config::Row, misc::retry, mux::set_mux_addr};

pub fn up(row: Row) {
    // match row.function {
    //     ButtonFunction::ChangePage => {
    //         config.read_page_data(u16::from_le_bytes(row.data[0..2].try_into().unwrap()));
    //         config.buttons.iter().enumerate().for_each(|(i, b)| {
    //             set_mux_addr(i as u8, &mut mux_pins);
    //             retry(|| display.draw(b.get_image()));
    //         });
    //     }
    //     _ => {
    //         debug!("short up");
    //     }
    // }
}
