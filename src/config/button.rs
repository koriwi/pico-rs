use super::action::ButtonFunction;
use super::IMAGE_SIZE;
use super::ROW_SIZE;

// -1 for the mode byte
const DATA_SIZE: usize = ROW_SIZE as usize / 2 - 1;

#[derive(Debug)]
pub struct Button {
    pub raw_image: [u8; IMAGE_SIZE as usize],
    pub raw_data: [u8; ROW_SIZE as usize],
}

impl Button {
    pub fn primary_function(&mut self) -> ButtonFunction {
        match self.raw_data[0] % 16 {
            0 => ButtonFunction::PressKeys(self.primary_data().into()),
            1 => ButtonFunction::ChangePage(self.primary_data().into()),
            3 => ButtonFunction::PressSpecialKey,
            4 => ButtonFunction::SendText,
            5 => ButtonFunction::SetSetting,
            6 => ButtonFunction::CommunicateToHost,
            _ => ButtonFunction::None, // invalid but also 2
        }
    }
    pub fn secondary_function(&mut self) -> ButtonFunction {
        match self.raw_data[ROW_SIZE as usize / 2] % 16 {
            0 => ButtonFunction::PressKeys(self.secondary_data().into()),
            1 => ButtonFunction::ChangePage(self.secondary_data().into()),
            3 => ButtonFunction::PressSpecialKey,
            4 => ButtonFunction::SendText,
            5 => ButtonFunction::SetSetting,
            6 => ButtonFunction::CommunicateToHost,
            _ => ButtonFunction::None, // invalid but also 2
        }
    }
    pub fn has_secondary_function(&self) -> bool {
        self.raw_data[ROW_SIZE as usize / 2] != 2
    }
    pub fn has_live_data(&mut self) -> bool {
        self.raw_image[0] == 1
    }

    pub fn image_buff(&self) -> &[u8] {
        &self.raw_image[1..]
    }
    pub fn primary_data(&self) -> &[u8] {
        &self.raw_data[1..DATA_SIZE + 1]
    }
    pub fn secondary_data(&self) -> &[u8] {
        &self.raw_data[DATA_SIZE + 2..]
    }
}
