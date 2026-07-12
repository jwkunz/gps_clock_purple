#![no_std]
#![no_main]

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

    ufmt::uwriteln!(&mut serial, "GPS clock starting!\r");
    display.write_string("12.34");

    loop {
        led.toggle();

        ufmt::uwriteln!(&mut serial, "Still alive...\r");

        arduino_hal::delay_ms(1000);
    }
}