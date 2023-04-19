use crate::{
    button::{Button, ButtonFunction},
    misc::NineTeenSeventy,
    read, BUTTON_COUNT,
};

use super::ROW_SIZE;
use defmt::debug;
use embedded_sdmmc::{Controller, Mode, SdMmcSpi, VolumeIdx};
#[derive(Debug)]
pub struct Header {
    width: u8,
    height: u8,
    offset: u16,
    row_size: u8,
    _page_count: u16,
}

impl From<[u8; ROW_SIZE]> for Header {
    fn from(header: [u8; ROW_SIZE]) -> Self {
        let offset = u16::from_le_bytes([header[2], header[3]]);
        Self {
            _page_count: (offset - 1) / BUTTON_COUNT as u16,
            width: header[0],
            height: header[1],
            offset,
            row_size: ROW_SIZE as u8,
        }
    }
}

impl Header {
    //.seek_from_start(header[2] as u32 * 128 + 1024 * db as u32)
    pub fn image_start(&self) -> u32 {
        (self.row_size as u16 * (self.offset)) as u32
    }
}

pub struct Row<'a> {
    pub data: &'a [u8],
    pub function: ButtonFunction,
}

pub type FDController<'a, SPI, CS> =
    Controller<embedded_sdmmc::BlockSpi<'a, SPI, CS>, NineTeenSeventy, 128, 128>;

pub struct Config<'a, SPI, CS>
where
    SPI: embedded_hal::blocking::spi::Transfer<u8>,
    CS: embedded_hal::digital::v2::OutputPin,
    <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
{
    header: Header,
    image_size: u32,
    config_file: embedded_sdmmc::File,
    volume: embedded_sdmmc::Volume,
    controller: FDController<'a, SPI, CS>,
    pub buttons: [Button; BUTTON_COUNT],
}

impl<'a, SPI, CS> Config<'a, SPI, CS>
where
    SPI: embedded_hal::blocking::spi::Transfer<u8>,
    CS: embedded_hal::digital::v2::OutputPin,
    <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
{
    pub fn new(spi_dev: &'a mut SdMmcSpi<SPI, CS>) -> Self {
        let mut controller: Controller<_, _, 128, 128> = match spi_dev.acquire() {
            Ok(block) => Controller::new(block, NineTeenSeventy {}),
            Err(e) => {
                debug!("{:?}", defmt::Debug2Format(&e));
                panic!("Failed to acquire SD card")
            }
        };
        let mut volume = controller.get_volume(VolumeIdx(0)).unwrap();
        let root_dir = controller.open_root_dir(&volume).unwrap();
        let mut config_file = controller
            .open_file_in_dir(&mut volume, &root_dir, "config.bin", Mode::ReadOnly)
            .unwrap();

        let mut header_buf = [0u8; ROW_SIZE];
        controller
            .read(&volume, &mut config_file, &mut header_buf)
            .unwrap();

        let header = Header::from(header_buf);
        let buttons = [Button::default(); BUTTON_COUNT];

        Self {
            buttons,
            config_file,
            volume,
            controller,
            header,
            image_size: 1024,
        }
    }

    fn read_button_image_data(&mut self, index: usize) {
        read!(self, buttons[index].raw_image).unwrap();
        self.buttons[index].parse_image();
    }

    fn read_button_function_data(&mut self, index: usize) {
        read!(self, buttons[index].raw_data).unwrap();
        debug!("----{}----", index);
        self.buttons[index].parse_functions();
    }

    pub fn read_page_data(&mut self, page: u16) {
        let bd_count = self.header.width * self.header.height;

        // adds 1 to skip the header
        let page_offset = (bd_count as u16 * page + 1) as u32 * self.header.row_size as u32;
        self.config_file.seek_from_start(page_offset).unwrap();

        for button_index in 0..self.buttons.len() {
            self.read_button_function_data(button_index);
        }

        let base_offset = self.header.image_start();
        let image_offset = base_offset + (self.image_size + 1) * (bd_count as u16 * page) as u32;
        self.config_file.seek_from_start(image_offset).unwrap();
        for button_index in 0..self.buttons.len() {
            self.read_button_image_data(button_index);
        }
    }

    pub fn get_primary_function(&self, index: usize) -> Row {
        Row {
            data: self.buttons[index].get_primary_data(),
            function: self.buttons[index].primary_function,
        }
    }
    pub fn get_secondary_function(&self, index: usize) -> Row {
        Row {
            data: self.buttons[index].get_secondary_data(),
            function: self.buttons[index].secondary_function,
        }
    }
}
