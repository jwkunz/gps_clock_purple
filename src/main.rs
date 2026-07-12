#![no_std]
#![no_main]

use heapless::String;
use core::fmt::Write;
use ufmt::uwrite;

use panic_halt as _;

use arduino_hal::prelude::*;

mod max7221;
use max7221::Max7221;

mod time_clock;
use time_clock::TimeClock;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let data = pins.d2.into_output();
    let clk  = pins.d4.into_output();
    let cs   = pins.d5.into_output();


    let mut display = Max7221::new(
        data,
        clk,
        cs
    );


display.init();

    // Create serial interface at 57600 baud
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut led = pins.d13.into_output();

    let pps = pins.d7.into_floating_input();

    ufmt::uwriteln!(&mut serial, "GPS clock starting!\r");

    let mut last_pps = false;
    let mut counter: u32 = 0;
    let mut text: String<16> = String::new();
    let mut clock = TimeClock::new(12, 0, 0, true);

    loop {
        let current_pps = pps.is_high();

        // Rising edge?
        if current_pps && !last_pps {
            text.clear();
            display.write_str(clock.format(&mut text));
            ufmt::uwriteln!(&mut serial, "{}", clock.format(&mut text)).unwrap();
            clock.tick_second();
        }

        last_pps = current_pps;
    }
}