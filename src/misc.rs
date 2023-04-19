use embedded_sdmmc::TimeSource;

pub struct NineTeenSeventy {}
impl TimeSource for NineTeenSeventy {
    fn get_timestamp(&self) -> embedded_sdmmc::Timestamp {
        embedded_sdmmc::Timestamp {
            year_since_1970: 0,
            zero_indexed_month: 0,
            zero_indexed_day: 0,
            hours: 0,
            minutes: 0,
            seconds: 0,
        }
    }
}

pub fn retry<F, T, E>(mut f: F) -> T
where
    F: FnMut() -> Result<T, E>,
    E: core::fmt::Debug,
{
    let mut result = f();
    while result.is_err() {
        result = f();
    }
    match result {
        Ok(t) => t,
        Err(e) => panic!("{:?}", e),
    }
}
