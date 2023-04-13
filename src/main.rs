#![no_std]
#![no_main]
mod button_machine;
mod mux;
mod pins;

extern crate alloc;

use rp_pico::hal;

// use the hal alias
use hal::{gpio::Pins, pac, sio::Sio, timer::Timer, Clock};

use alloc::boxed::Box;
use alloc_cortex_m::CortexMHeap;
use cortex_m::delay::Delay;
use cortex_m_rt::entry;
use defmt::debug;
use defmt_rtt as _;
use panic_probe as _;

use crate::{
    mux::{get_mux_pins, set_mux_addr},
    pins::create_pin_array,
};
#[global_allocator]
static ALLOCATOR: CortexMHeap = CortexMHeap::empty();

#[entry]
fn main() -> ! {
    unsafe {
        ALLOCATOR.init(cortex_m_rt::heap_start() as usize, 1024);
    }
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();

    // Set up the watchdog driver - needed by the clock setup code
    let mut watchdog = hal::Watchdog::new(pac.WATCHDOG);

    // The default is to generate a 125 MHz system clock
    let clocks = hal::clocks::init_clocks_and_plls(
        rp_pico::XOSC_CRYSTAL_FREQ,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
    .ok()
    .unwrap();
    let mut delay = Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());
    let sio = Sio::new(pac.SIO);
    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut pin_array = create_pin_array(pins);

    // get pins programmatically by index
    let mut mux_pins = get_mux_pins(&mut pin_array);

    let mut button_pin = pin_array[19].take().unwrap();
    button_pin.into_pull_up_input();

    let timer = Timer::new(pac.TIMER, &mut pac.RESETS);
    let mut button_machine = button_machine::ButtonMachine::new(
        &button_pin,
        200,
        &timer,
        Box::new(|action, index| {
            debug!("action: {}: {:?}", index, action);
        }),
    );
    let mut button_index = 0;
    loop {
        if timer.get_counter().ticks() % 1000 == 0 {
            set_mux_addr(button_index, &mut mux_pins);
            delay.delay_us(10); // wait for mux to settle

            button_machine.check_button(button_index, true).unwrap();
            button_index += 1;
            if button_index > 7 {
                button_index = 0;
            }
        }
        // if serial.line_coding().data_rate() == 1200 {
        //     // Reset the board if the host sets the baud rate to 1200
        //     hal::rom_data::reset_to_usb_boot(0, 0);
        // }
        // usb_dev.poll(&mut [&mut serial]);
    }
}
