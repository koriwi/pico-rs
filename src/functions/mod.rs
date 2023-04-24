use ssd1306::prelude::WriteOnlyDataCommand;
use ssd1306::size::DisplaySize;
use ssd1306::Ssd1306;

use crate::config::Config;
use crate::config::RWSeek;
use crate::util::retry;

pub struct Functions<'a, C, DI, SIZE, MODE> {
    config: &'a mut Config<C>,
    display: &'a mut Ssd1306<DI, SIZE, MODE>,
    set_mux_addr: &'a mut dyn FnMut(u8),
    button_index: &'a mut usize,
}

impl<'a, C, DI, SIZE, MODE> Functions<'a, C, DI, SIZE, MODE>
where
    C: RWSeek,
    DI: WriteOnlyDataCommand,
    SIZE: DisplaySize,
{
    pub fn new(
        config: &'a mut Config<C>,
        display: &'a mut Ssd1306<DI, SIZE, MODE>,
        set_mux_addr: &'a mut dyn FnMut(u8),
        button_index: &'a mut usize,
    ) -> Self {
        Self {
            config,
            display,
            set_mux_addr,
            button_index,
        }
    }

    pub fn none(&mut self) {
        *self.button_index += 1;
        if *self.button_index > 7 {
            *self.button_index = 0;
        }
        (self.set_mux_addr)(*self.button_index as u8);
    }

    pub fn change_page(&mut self, target_page: u16) {
        self.config.load_page(target_page);
        for (i, button) in self.config.page.buttons.iter().enumerate() {
            (self.set_mux_addr)(i as u8);
            retry(|| self.display.draw(button.image_buff()));
        }
    }
}
