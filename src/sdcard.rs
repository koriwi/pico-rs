use embedded_hal::spi::MODE_0;
use embedded_sdmmc::{Controller, File, Mode, SdMmcSpi, Volume, VolumeIdx};
use fugit::{HertzU32, RateExtU32};
use rp_pico::{
    hal::{
        gpio::{DynFunction, DynPin, DynPinMode},
        spi::{Enabled, SpiDevice},
        Spi,
    },
    pac::RESETS,
};

use crate::{config::RWSeek, debug, util::NineTeenSeventy};

pub struct SpiPins {
    data_pins: [DynPin; 3],
    cs: DynPin,
}

impl SpiPins {
    pub fn new(mosi: DynPin, miso: DynPin, sclk: DynPin, cs: DynPin) -> Self {
        Self {
            data_pins: [mosi, miso, sclk],
            cs,
        }
    }
}

pub fn create_sdcard<D>(
    spi: D,
    mut pins: SpiPins,
    freq: HertzU32,
    speed: u32,
    reset: &mut RESETS,
) -> SdMmcSpi<Spi<Enabled, D, 8>, DynPin>
where
    D: SpiDevice,
{
    pins.data_pins.iter_mut().for_each(|pin| {
        pin.try_into_mode(DynPinMode::Function(DynFunction::Spi))
            .unwrap();
    });
    pins.cs.into_push_pull_output();

    let spi_disabled = Spi::<_, _, 8>::new(spi);
    let spi = spi_disabled.init(reset, freq, speed.MHz(), &MODE_0);

    SdMmcSpi::new(spi, pins.cs)
}

pub struct SDConfigFile<C> {
    controller: C,
    volume: Volume,
    file: File,
}

impl<'a, SPI, CS>
    SDConfigFile<Controller<embedded_sdmmc::BlockSpi<'a, SPI, CS>, NineTeenSeventy, 128, 128>>
where
    SPI: embedded_hal::blocking::spi::Transfer<u8>,
    CS: embedded_hal::digital::v2::OutputPin,
    <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
{
    pub fn new(spi_dev: &'a mut SdMmcSpi<SPI, CS>) -> Self
    where
        SPI: embedded_hal::blocking::spi::Transfer<u8>,
        CS: embedded_hal::digital::v2::OutputPin,
        <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
    {
        let mut controller: Controller<_, _, 128, 128> = match spi_dev.acquire() {
            Ok(block) => Controller::new(block, NineTeenSeventy {}),
            Err(e) => {
                debug!("{:?}", defmt::Debug2Format(&e));
                panic!("Failed to acquire SD card")
            }
        };
        let mut volume = controller.get_volume(VolumeIdx(0)).unwrap();
        let root_dir = controller.open_root_dir(&volume).unwrap();
        let config_file = controller
            .open_file_in_dir(&mut volume, &root_dir, "config.bin", Mode::ReadOnly)
            .unwrap();

        Self {
            controller,
            volume,
            file: config_file,
        }
    }
}

impl<'a, SPI, CS> RWSeek
    for SDConfigFile<Controller<embedded_sdmmc::BlockSpi<'a, SPI, CS>, NineTeenSeventy, 128, 128>>
where
    SPI: embedded_hal::blocking::spi::Transfer<u8>,
    CS: embedded_hal::digital::v2::OutputPin,
    <SPI as embedded_hal::blocking::spi::Transfer<u8>>::Error: core::fmt::Debug,
{
    fn seek_from_start(&mut self, pos: u32) -> Result<(), ()> {
        self.file.seek_from_start(pos).unwrap();
        Ok(())
    }
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, ()> {
        let read = self
            .controller
            .read(&mut self.volume, &mut self.file, buf)
            .unwrap();
        Ok(read)
    }
    fn write(&mut self, buf: &[u8]) -> Result<usize, ()> {
        todo!()
    }
}
