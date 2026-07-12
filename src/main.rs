#![no_std]
#![no_main]

use panic_halt as _;

use arduino_hal::prelude::*;

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    // Create serial interface at 57600 baud
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    let mut led = pins.d13.into_output();

    ufmt::uwriteln!(&mut serial, "GPS clock starting!\r");

    loop {
        led.toggle();

        ufmt::uwriteln!(&mut serial, "Still alive...\r");

        arduino_hal::delay_ms(1000);
    }
}