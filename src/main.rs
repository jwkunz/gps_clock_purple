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

mod time_input;

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

    let mut last_pps = false;
    let mut counter: u32 = 0;
    let mut text: String<16> = String::new();


    let mut clock = TimeClock::new(
        0,
        0,
        0
    );
    let mut delay = arduino_hal::Delay::new();
    

    let mut input: String<8> = String::new();

    loop {
        input.clear();

        ufmt::uwriteln!(
            &mut serial,
            "Enter 24 hour time HH:MM:SS:"
        ).unwrap();

        loop {
            let c = nb::block!(serial.read()).unwrap();

            match c {
                b'\r' | b'\n' => break,

                _ => {
                    if input.len() < 8 {
                        input.push(c as char).ok();
                    }
                }
            }
        }

        if let Some((h, m, s)) = time_input::parse_time(input.as_str()) {
            clock.set_time(h, m, s, 0);
            break;
        }

        ufmt::uwriteln!(
            &mut serial,
            "Invalid. Try again."
        ).unwrap();
    }


    ufmt::uwriteln!(&mut serial, "GPS clock starting!\r");
    loop {
        let current_pps = pps.is_high();

        // Rising edge?
        if current_pps && !last_pps {
            text.clear();
            ufmt::uwriteln!(&mut serial, "{}", clock.format(&mut text)).unwrap();
            clock.tick_pps();
        }

        clock.increment_millis_no_tick(10);
        delay.delay_us(9150u16);
        display.write_str(clock.format(&mut text));

        last_pps = current_pps;
    }
}