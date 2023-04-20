use crate::debug;

pub enum ButtonFunction<'a> {
    PressKeys(PressKeys<'a>), //0
    ChangePage(ChangePage),   //1
    None,                     //2
    PressSpecialKey,          //3
    SendText,                 //4
    SetSetting,               //5
    CommunicateToHost,        //6
}
pub struct ChangePage {
    pub target_page: u16,
}

impl From<&[u8]> for ChangePage {
    fn from(value: &[u8]) -> Self {
        Self {
            target_page: u16::from_le_bytes(value[0..2].try_into().unwrap()),
        }
    }
}

pub struct PressKeys<'a> {
    pub keys: &'a [u8],
    pub goto: Option<u16>,
}

impl<'a> From<&'a [u8]> for PressKeys<'a> {
    fn from(value: &'a [u8]) -> Self {
        debug!("PressKeys::from({:?})", value);
        let len = value.len();
        let last_key_index = value.iter().position(|&k| k == 0).unwrap();
        let keys = &value[..last_key_index];
        let goto = match u16::from_le_bytes(value[len - 3..len - 1].try_into().unwrap()) {
            0 => None,
            p => Some(p - 1),
        };
        Self { keys, goto }
    }
}
