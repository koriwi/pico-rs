use crate::BUTTON_COUNT;

use super::button::Button;
use super::DataBuffs;
use super::ImagesBuffs;

pub struct Page {
    pub buttons: [Button; BUTTON_COUNT],
}

impl From<(DataBuffs, ImagesBuffs)> for Page {
    fn from((data_buffs, images_buffs): (DataBuffs, ImagesBuffs)) -> Self {
        let buttons = core::array::from_fn::<_, BUTTON_COUNT, _>(|idx| Button {
            raw_data: data_buffs[idx],
            raw_image: images_buffs[idx],
        });

        Self { buttons }
    }
}
