use embedded_hal::spi::MODE_0;
use embedded_sdmmc::SdMmcSpi;
use fugit::{HertzU32, RateExtU32};
use rp_pico::{
    hal::{
        gpio::{DynFunction, DynPin, DynPinMode, Pin},
        spi::{Enabled, SpiDevice},
        Spi,
    },
    pac::RESETS,
};

#[macro_export]
macro_rules! SD_SPI {
    ($pins:ident, $pac:ident, $clocks:ident, $($pin:ident),*) => {{
        $(let _spi_sclk = $pins.$pin.into_mode::<hal::gpio::FunctionSpi>();)*
        let spi_disabled = Spi::<_, _, 8>::new($pac.SPI1);
        let spi = spi_disabled.init(
            &mut $pac.RESETS,
            $clocks.system_clock.freq(),
            SD_MHZ.MHz(),
            &MODE_0,
        );
        let sd_cs = $pins.gpio9.into_push_pull_output();
        SdMmcSpi::new(spi, sd_cs)
    }
    };
}

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
