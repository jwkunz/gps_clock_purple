#![no_std]
#![no_main]

use heapless::String;
use core::fmt::Write;
use ufmt::uwrite;

use panic_halt as _;

use arduino_hal::prelude::*;

mod max7221;

use max7221::Max7221;

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
    let mut text: String<8> = String::new();

    loop {
        let current_pps = pps.is_high();

        // Rising edge?
        if current_pps && !last_pps {
            led.toggle();

            text.clear();
            write!(&mut text, "{}", counter).unwrap();

            ufmt::uwriteln!(&mut serial, "Elapsed Seconds: {}", counter).unwrap();

            display.write_str(&text);

            counter += 1;
        }

        last_pps = current_pps;
    }
}