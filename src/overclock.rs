use fugit::{HertzU32, RateExtU32};
use rp_pico::{
    hal::{
        clocks::{ClocksManager, InitError},
        pll::{common_configs::PLL_USB_48MHZ, setup_pll_blocking, PLLConfig},
        xosc::setup_xosc_blocking,
        Watchdog,
    },
    pac::{CLOCKS, PLL_SYS, PLL_USB, RESETS, XOSC},
};

const PLL_OVERCLOCK_SYS: PLLConfig = PLLConfig {
    vco_freq: HertzU32::MHz(1500),
    refdiv: 1,
    post_div1: 6,
    post_div2: 2,
};

pub fn init_clocks_and_plls(
    xosc_crystal_freq: u32,
    xosc_dev: XOSC,
    clocks_dev: CLOCKS,
    pll_sys_dev: PLL_SYS,
    pll_usb_dev: PLL_USB,
    resets: &mut RESETS,
    watchdog: &mut Watchdog,
) -> Result<ClocksManager, InitError> {
    let xosc = setup_xosc_blocking(xosc_dev, xosc_crystal_freq.Hz()).map_err(InitError::XoscErr)?;

    // Configure watchdog tick generation to tick over every microsecond
    watchdog.enable_tick_generation((xosc_crystal_freq / 1_000_000) as u8);

    let mut clocks = ClocksManager::new(clocks_dev);

    let pll_sys = setup_pll_blocking(
        pll_sys_dev,
        xosc.operating_frequency(),
        PLL_OVERCLOCK_SYS,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;
    let pll_usb = setup_pll_blocking(
        pll_usb_dev,
        xosc.operating_frequency(),
        PLL_USB_48MHZ,
        &mut clocks,
        resets,
    )
    .map_err(InitError::PllError)?;

    clocks
        .init_default(&xosc, &pll_sys, &pll_usb)
        .map_err(InitError::ClockError)?;
    Ok(clocks)
}
