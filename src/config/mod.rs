pub mod action;
pub mod button;
pub mod header;
pub mod page;

use crate::debug;
use crate::BUTTON_COUNT;

use header::Header;
use page::Page;

const ROW_SIZE: u32 = 128;
const IMAGE_SIZE: u32 = 1025;

type DataBuffs = [[u8; ROW_SIZE as usize]; BUTTON_COUNT];
type ImagesBuffs = [[u8; IMAGE_SIZE as usize]; BUTTON_COUNT];

pub trait RWSeek {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()>;
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()>;
    fn seek_from_start(&mut self, pos: u32) -> Result<(), ()>;
}

pub struct Config<C> {
    pub header: Header,
    config_file: C,
    pub page: Page,
}

impl<C> Config<C>
where
    C: RWSeek,
{
    pub fn new(mut config_file: C) -> Self {
        let mut header_buf = [0u8; ROW_SIZE as usize];
        config_file.read(&mut header_buf).unwrap();

        let header = Header::from(header_buf);
        let page = Self::load_page_from_file(&mut config_file, &header, 0);

        Self {
            page,
            config_file,
            header,
        }
    }
    fn load_page_from_file(config_file: &mut C, header: &Header, page: u16) -> Page
    where
        C: RWSeek,
    {
        let mut data_buffs: DataBuffs = [[0u8; ROW_SIZE as usize]; BUTTON_COUNT];
        let data_offset = header.data_offset(page);
        debug!("data_offset: {}", data_offset);
        config_file.seek_from_start(data_offset).unwrap();
        for button_index in 0..BUTTON_COUNT {
            config_file.read(&mut data_buffs[button_index]).unwrap();
        }

        let mut images_buffs: ImagesBuffs = [[0u8; IMAGE_SIZE as usize]; BUTTON_COUNT];
        let images_offset = header.images_offset(page);
        config_file.seek_from_start(images_offset).unwrap();
        for button_index in 0..BUTTON_COUNT {
            config_file.read(&mut images_buffs[button_index]).unwrap();
        }
        Page::from((data_buffs, images_buffs))
    }
    pub fn load_page(&mut self, page: u16) {
        self.page = Self::load_page_from_file(&mut self.config_file, &self.header, page)
    }
}
