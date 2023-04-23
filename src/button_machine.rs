pub use button_machine::*;
use defmt::Format;
use embedded_hal::digital::v2::InputPin;
use finite_state_machine::state_machine;

use crate::config::button::Button;

use super::hal;
use fugit::Instant;
use hal::gpio::DynPin;

#[derive(Format)]
pub enum ButtonEvent {
    ShortDown,
    ShortUp,
    ShortTriggered,
    LongTriggered,
}

pub struct Data<'a> {
    down_at: Option<Instant<u64, 1, 1000000>>,
    button: &'a mut Button,
    execute: &'a mut dyn FnMut(Option<ButtonEvent>, &mut Button),
}

pub struct Config<'a> {
    pin: &'a DynPin,
    long_press_duration: u64,
    timer: &'a hal::Timer,
}

state_machine!(
    ButtonMachine(Config<'a>, Data<'b>);
    Start {
        IsUp => End,
        IsDown => Down
    },
    Down {
        StillDown => Down,
        AnyUp => Up,
        HeldLong => DownButWaiting
    },
    DownButWaiting {
        StillDown => DownButWaiting,
        Released => Up
    },
    Up {
        UpAfterLong => End,
        ShortUp => End,
        ShortHeld => End
    }
);

enum ButtonState {
    Up,
    Down,
}

impl<'a> ButtonMachine<'a> {
    pub fn new(
        pin: &'a DynPin,
        long_press_duration: u64,
        timer: &'a hal::Timer,
    ) -> ButtonMachine<'a> {
        ButtonMachine {
            config: Config {
                pin,
                long_press_duration,
                timer,
            },
        }
    }
    fn get_button_state(&self) -> ButtonState {
        let is_down = match self.config.pin.is_low() {
            Ok(state) => state,
            Err(_) => {
                return ButtonState::Up;
            }
        };
        if is_down {
            ButtonState::Down
        } else {
            ButtonState::Up
        }
    }
    fn get_now(&self) -> Instant<u64, 1, 1000000> {
        Instant::<u64, 1, 1_000_000>::from_ticks(self.config.timer.get_counter().ticks())
    }
    fn get_diff_since_down(&self, data: &Data) -> u64 {
        let down_at = data.down_at.unwrap();
        let now = self.get_now();
        now.checked_duration_since(down_at).unwrap().to_millis()
    }
    pub fn check_button<'b>(
        &mut self,
        button: &'b mut Button,
        execute: &mut dyn FnMut(Option<ButtonEvent>, &mut Button),
    ) -> Result<(), &'static str> {
        let mut state = Data {
            down_at: None,
            button,
            execute,
        };
        self.run_to_end(&mut state)?;
        Ok(())
    }
}

impl<'a, 'b> Deciders<Data<'b>> for ButtonMachine<'a> {
    fn start(&self, data: &Data) -> StartEvents {
        match self.get_button_state() {
            ButtonState::Down => StartEvents::IsDown,
            ButtonState::Up => StartEvents::IsUp,
        }
    }
    fn down(&self, data: &Data) -> DownEvents {
        // todo: implement wakeup from screensaver
        match data.button.has_secondary_function() {
            true => {
                let diff = self.get_diff_since_down(data);
                if diff > self.config.long_press_duration {
                    return DownEvents::HeldLong;
                }
                match self.get_button_state() {
                    ButtonState::Down => DownEvents::StillDown,
                    ButtonState::Up => DownEvents::AnyUp,
                }
            }
            false => match self.get_button_state() {
                ButtonState::Down => DownEvents::StillDown,
                ButtonState::Up => DownEvents::AnyUp,
            },
        }
    }
    fn up(&self, data: &Data) -> UpEvents {
        match data.button.has_secondary_function() {
            true => {
                let diff = self.get_diff_since_down(data);
                if diff > self.config.long_press_duration {
                    return UpEvents::UpAfterLong;
                }
                UpEvents::ShortHeld
            }
            false => UpEvents::ShortUp,
        }
    }
    fn down_but_waiting(&self, data: &Data) -> DownButWaitingEvents {
        match self.get_button_state() {
            ButtonState::Down => DownButWaitingEvents::StillDown,
            ButtonState::Up => DownButWaitingEvents::Released,
        }
    }
}

impl<'a, 'b> StartTransitions<Data<'b>> for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn is_down(&mut self, data: &mut Data) -> Result<(), &'static str> {
        data.down_at = Some(self.get_now());
        match data.button.has_secondary_function() {
            false => {
                (data.execute)(Some(ButtonEvent::ShortDown), data.button);
            }
            true => {}
        }
        Ok(())
    }
    fn is_up(&mut self, data: &mut Data) -> Result<(), &'static str> {
        (data.execute)(None, data.button);
        Ok(())
    }
}

impl<'a, 'b> DownTransitions<Data<'b>> for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn held_long(&mut self, data: &mut Data) -> Result<(), &'static str> {
        // let start = Instant::<u64, 1, 1_000_000>::from_ticks(self.config.timer.get_counter().ticks());
        (data.execute)(Some(ButtonEvent::LongTriggered), data.button);
        // let end = Instant::<u64, 1, 1_000_000>::from_ticks(self.config.timer.get_counter().ticks());
        // let diff = (end - start).to_micros();
        // debug!("long press took {}us", diff);
        Ok(())
    }
    fn any_up(&mut self, data: &mut Data) -> Result<(), &'static str> {
        Ok(())
    }
    fn still_down(&mut self, data: &mut Data) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a, 'b> DownButWaitingTransitions<Data<'b>> for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn released(&mut self, data: &mut Data) -> Result<(), &'static str> {
        Ok(())
    }
    fn still_down(&mut self, data: &mut Data) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a, 'b> UpTransitions<Data<'b>> for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn short_held(&mut self, data: &mut Data) -> Result<(), &'static str> {
        (data.execute)(Some(ButtonEvent::ShortTriggered), data.button);
        data.down_at = None;
        Ok(())
    }
    fn short_up(&mut self, data: &mut Data) -> Result<(), &'static str> {
        (data.execute)(Some(ButtonEvent::ShortUp), data.button);
        data.down_at = None;
        Ok(())
    }
    fn up_after_long(&mut self, data: &mut Data) -> Result<(), &'static str> {
        data.down_at = None;
        Ok(())
    }
}
