use defmt::{debug, Debug2Format};
use embedded_sdmmc::BlockDevice;

use crate::{config::FDController, read, ROW_SIZE};

#[derive(Debug, Clone, Copy)]
pub enum ButtonFunction {
    PressKeys,         //0
    ChangePage,        //1
    None,              //2
    PressSpecialKey,   //3
    SendText,          //4
    SetSetting,        //5
    CommunicateToHost, //6
}

const DATA_SIZE: usize = ROW_SIZE / 2 - 1;

#[derive(Debug, Clone, Copy)]
pub struct Button {
    pub raw_image: [u8; 1025],
    pub raw_data: [u8; ROW_SIZE],
    pub has_live_data: bool,
    pub primary_function: ButtonFunction,
    pub secondary_function: ButtonFunction,
}

impl Default for Button {
    fn default() -> Self {
        let raw_data = [0u8; ROW_SIZE];
        Self {
            raw_image: [255u8; 1025],
            raw_data,
            has_live_data: false,
            primary_function: ButtonFunction::None,
            secondary_function: ButtonFunction::None,
        }
    }
}

impl Button {
    pub fn parse_functions(&mut self) {
        self.primary_function = match self.raw_data[0] % 16 {
            0 => ButtonFunction::PressKeys,
            1 => ButtonFunction::ChangePage,
            3 => ButtonFunction::PressSpecialKey,
            4 => ButtonFunction::SendText,
            5 => ButtonFunction::SetSetting,
            6 => ButtonFunction::CommunicateToHost,
            _ => ButtonFunction::None, // invalid but also 2
        };
        self.secondary_function = match self.raw_data[ROW_SIZE / 2] % 16 {
            0 => ButtonFunction::PressKeys,
            1 => ButtonFunction::ChangePage,
            3 => ButtonFunction::PressSpecialKey,
            4 => ButtonFunction::SendText,
            5 => ButtonFunction::SetSetting,
            6 => ButtonFunction::CommunicateToHost,
            _ => ButtonFunction::None, // invalid but also 2
        };
        debug!(
            "1: {:?} raw: {}",
            Debug2Format(&self.primary_function),
            self.raw_data[0]
        );
        debug!(
            "2: {:?} raw:{}",
            Debug2Format(&self.secondary_function),
            self.raw_data[ROW_SIZE / 2]
        );
    }

    pub fn parse_image(&mut self) {
        self.has_live_data = self.raw_image[0] == 1;
    }

    pub fn get_image(&self) -> &[u8] {
        &self.raw_image[1..]
    }
    pub fn get_primary_data(&self) -> &[u8] {
        &self.raw_data[1..DATA_SIZE]
    }
    pub fn get_secondary_data(&self) -> &[u8] {
        &self.raw_data[DATA_SIZE + 1..]
    }
}
