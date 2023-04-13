extern crate alloc;
pub use button_machine::*;
use defmt::Format;
use embedded_hal::digital::v2::InputPin;
use finite_state_machine::state_machine;

use alloc::boxed::Box;
use fugit::Instant;
use rp_pico::hal::{self, gpio::DynPin};

#[derive(Format)]
pub enum Actions {
    ShortDown,
    ShortUp,
    LongTriggered,
}

pub struct Data<'a> {
    pin: &'a DynPin,
    down_at: Option<Instant<u64, 1, 1000000>>,
    has_long_press: bool,
    long_press_duration: u64,
    timer: &'a hal::Timer,
    execute: Box<dyn FnMut(Actions)>,
}

state_machine!(
    ButtonMachine(Data<'a>);
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
        execute: Box<dyn FnMut(Actions)>,
    ) -> ButtonMachine<'a> {
        ButtonMachine {
            data: Data {
                pin,
                down_at: None,
                has_long_press: false,
                long_press_duration,
                timer,
                execute,
            },
            state: State::Start,
        }
    }
    fn get_button_state(&self) -> ButtonState {
        let is_down = match self.data.pin.is_low() {
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
        Instant::<u64, 1, 1_000_000>::from_ticks(self.data.timer.get_counter().ticks())
    }
    fn get_diff_since_down(&self) -> u64 {
        let down_at = self.data.down_at.unwrap();
        let now = self.get_now();
        now.checked_duration_since(down_at).unwrap().to_millis()
    }
    pub fn check_button(&mut self, has_long_press: bool) -> Result<(), &'static str> {
        self.state = State::Start;
        self.data.has_long_press = has_long_press;
        self.run_to_end()?;
        Ok(())
    }
}

impl<'a> Deciders for ButtonMachine<'a> {
    fn start(&self) -> StartEvents {
        match self.get_button_state() {
            ButtonState::Down => StartEvents::IsDown,
            ButtonState::Up => StartEvents::IsUp,
        }
    }
    fn down(&self) -> DownEvents {
        // todo: implement wakeup from screensaver
        match self.data.has_long_press {
            true => {
                let diff = self.get_diff_since_down();
                if diff > self.data.long_press_duration {
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
    fn up(&self) -> UpEvents {
        match self.data.has_long_press {
            true => {
                let diff = self.get_diff_since_down();
                if diff > self.data.long_press_duration {
                    return UpEvents::UpAfterLong;
                }
                UpEvents::ShortHeld
            }
            false => UpEvents::ShortUp,
        }
    }
    fn down_but_waiting(&self) -> DownButWaitingEvents {
        match self.get_button_state() {
            ButtonState::Down => DownButWaitingEvents::StillDown,
            ButtonState::Up => DownButWaitingEvents::Released,
        }
    }
}

impl<'a> StartTransitions for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn is_down(&mut self) -> Result<(), &'static str> {
        self.data.down_at = Some(self.get_now());
        match self.data.has_long_press {
            false => {
                (self.data.execute)(Actions::ShortDown);
            }
            true => {}
        }
        Ok(())
    }
    fn is_up(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a> DownTransitions for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn held_long(&mut self) -> Result<(), &'static str> {
        // let start = Instant::<u64, 1, 1_000_000>::from_ticks(self.data.timer.get_counter().ticks());
        (self.data.execute)(Actions::LongTriggered);
        // let end = Instant::<u64, 1, 1_000_000>::from_ticks(self.data.timer.get_counter().ticks());
        // let diff = (end - start).to_micros();
        // debug!("long press took {}us", diff);
        Ok(())
    }
    fn any_up(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn still_down(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a> DownButWaitingTransitions for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn released(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
    fn still_down(&mut self) -> Result<(), &'static str> {
        Ok(())
    }
}

impl<'a> UpTransitions for ButtonMachine<'a> {
    fn illegal(&mut self) {}
    fn short_held(&mut self) -> Result<(), &'static str> {
        (self.data.execute)(Actions::ShortDown);
        (self.data.execute)(Actions::ShortUp);
        self.data.down_at = None;
        Ok(())
    }
    fn short_up(&mut self) -> Result<(), &'static str> {
        (self.data.execute)(Actions::ShortUp);
        self.data.down_at = None;
        Ok(())
    }
    fn up_after_long(&mut self) -> Result<(), &'static str> {
        self.data.down_at = None;
        Ok(())
    }
}
