use super::IMAGE_SIZE;
use super::ROW_SIZE;

const HEADER_SIZE: u32 = ROW_SIZE;

#[derive(Debug)]
pub struct Header {
    pub width: u8,
    pub height: u8,
    pub bd_count: u32,
    pub page_count: u16,
    offset: u16,
}

impl From<[u8; ROW_SIZE as usize]> for Header {
    fn from(header: [u8; ROW_SIZE as usize]) -> Self {
        let width = header[0];
        let height = header[1];
        let bd_count = width as u32 * height as u32;

        let offset = u16::from_le_bytes([header[2], header[3]]);
        let page_count = offset / (width * height) as u16;

        Self {
            bd_count,
            width,
            height,
            offset,
            page_count,
        }
    }
}

impl Header {
    pub fn data_offset(&self, page: u16) -> u32 {
        ROW_SIZE * self.bd_count * page as u32 + HEADER_SIZE
    }
    pub fn images_offset(&self, page: u16) -> u32 {
        self.offset as u32 * ROW_SIZE + IMAGE_SIZE * self.bd_count * (page) as u32
    }
}
