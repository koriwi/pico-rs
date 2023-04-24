use defmt::Debug2Format;
use ssd1306::prelude::WriteOnlyDataCommand;
use ssd1306::size::DisplaySize;
use ssd1306::Ssd1306;

use crate::button_machine::ButtonEvent;
use crate::config::action::ButtonFunction;
use crate::config::page::Page;
use crate::config::Config;
use crate::config::RWSeek;
use crate::debug;
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

    fn none(&mut self) {
        *self.button_index += 1;
        if *self.button_index > 7 {
            *self.button_index = 0;
        }
        (self.set_mux_addr)(*self.button_index as u8);
    }

    fn change_page(&mut self, target_page: u16) {
        self.config.load_page(target_page);
        for (i, button) in self.config.page.buttons.iter().enumerate() {
            (self.set_mux_addr)(i as u8);
            retry(|| self.display.draw(button.image_buff()));
        }
        (self.set_mux_addr)(*self.button_index as u8);
    }

    pub fn has_secondary_function(&self) -> bool {
        let button = &self.config.page.buttons[*self.button_index];
        button.has_secondary_function()
    }

    pub fn bar(&mut self, event: Option<ButtonEvent>) {
        let button = &mut self.config.page.buttons[*self.button_index];
        let event = match event {
            Some(a) => a,
            None => {
                self.none();
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
                ButtonEvent::LongTriggered | ButtonEvent::ShortTriggered | ButtonEvent::ShortDown,
            ) => {
                self.change_page(data.target_page);
            }
            _ => {}
        };
    }
}
