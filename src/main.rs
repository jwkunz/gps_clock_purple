#![no_std]
#![no_main]
#![feature(abi_avr_interrupt)]

use core::cell::Cell;
use heapless::String;

use panic_halt as _;

mod max7221;
use max7221::Max7221;

mod nmea;
mod soft_uart;
use soft_uart::SoftUart;

mod time_clock;
use time_clock::TimeClock;

mod time_zone;

// Set only by the PPS ISR.  The main loop consumes it before starting an NMEA
// capture; the receiver also watches it to abort if another pulse arrives.
static PPS_PENDING: avr_device::interrupt::Mutex<Cell<bool>> =
    avr_device::interrupt::Mutex::new(Cell::new(false));

const TIMER_TICKS_PER_CENTISECOND: u16 = 625;
const DISPLAY_QUIET_AFTER_CENTISECONDS: u8 = 98;

pub(crate) fn pps_pending() -> bool {
    avr_device::interrupt::free(|cs| PPS_PENDING.borrow(cs).get())
}

fn take_pps() -> bool {
    avr_device::interrupt::free(|cs| PPS_PENDING.borrow(cs).replace(false))
}

/// Timer1 ticks every 16 µs at the selected /256 prescaler.  It is reset by
/// PPS, so 625 ticks represent one centisecond.
fn centiseconds_since_pps() -> Option<u8> {
    avr_device::interrupt::free(|_| {
        let timer1 = unsafe { arduino_hal::pac::TC1::steal() };

        // A missing PPS lets the 16-bit timer overflow after about 1.049 s.
        // Do not present a wrapped counter as a synchronized fraction.
        if timer1.tifr1().read().tov1().bit_is_set() {
            return None;
        }

        Some((timer1.tcnt1().read().bits() / TIMER_TICKS_PER_CENTISECOND) as u8)
    })
}

#[avr_device::interrupt(atmega328p)]
fn PCINT2() {
    // Pin-change interrupts fire on both PPS edges.  D7 is PD7; only retain
    // the active-high edge.  Direct register access keeps the ISR tiny.
    let port_d = unsafe { arduino_hal::pac::PORTD::steal().pind().read().bits() };
    if port_d & (1 << 7) != 0 {
        let timer1 = unsafe { arduino_hal::pac::TC1::steal() };
        timer1.tcnt1().write(|w| w.set(0));
        // AVR timer flags are cleared by writing a one to their bit.
        timer1.tifr1().write(|w| w.tov1().set_bit());
        avr_device::interrupt::free(|cs| PPS_PENDING.borrow(cs).set(true));
    }
}

#[arduino_hal::entry]
fn main() -> ! {
    let dp = arduino_hal::Peripherals::take().unwrap();
    let pins = arduino_hal::pins!(dp);

    let data = pins.d2.into_output();
    let clk = pins.d4.into_output();
    let cs = pins.d5.into_output();
    let mut display = Max7221::new(data, clk, cs);
    display.init();

    // D7 = GPS PPS (PD7 / PCINT23), D8 = GPS TX (PB0, idle-high 9600 8-N-1).
    let _pps = pins.d7.into_floating_input();
    let gps_rx = pins.d8.into_floating_input();
    let mut gps_uart = SoftUart::new(gps_rx);

    let mut text: String<16> = String::new();
    let mut clock = TimeClock::new(0, 0, 0);
    let mut primed_time: Option<(u8, u8, u8)> = None;
    let mut last_centiseconds = u8::MAX;
    let mut synchronized = false;

    // Timer1 runs freely at 16 MHz / 256 = 62,500 Hz.  One second is 62,500
    // counts, so it never overflows while valid PPS pulses are present.
    dp.TC1.tccr1a().write(|w| unsafe { w.bits(0) });
    dp.TC1.tccr1b().write(|w| w.cs1().prescale_256());
    dp.TC1.tcnt1().write(|w| w.set(0));
    dp.TC1.tifr1().write(|w| w.tov1().set_bit());

    // PCINT2 covers port D.  PCINT23 corresponds to PD7 / Arduino D7.
    dp.EXINT.pcicr().write(|w| unsafe { w.bits(0b100) });
    dp.EXINT.pcmsk2().write(|w| w.set(1 << 7));
    unsafe { avr_device::interrupt::enable() };

    display.write_str(clock.format(&mut text, 0));

    loop {
        if take_pps() {
            synchronized = true;
            last_centiseconds = u8::MAX;

            // An RMC sentence received after this edge is intentionally
            // applied at the *next* edge.  This accepts the GPS module's
            // known one-second NMEA lag while keeping seconds PPS-aligned.
            if let Some((hours, minutes, seconds)) = primed_time.take() {
                clock.set_time(hours, minutes, seconds);
            } else {
                clock.tick_second();
            }

            // Capture before doing display I/O: the first UART start bit may
            // begin immediately with PPS.  The ISR only records a next pulse.
            primed_time = gps_uart
                .capture_rmc_time()
                .map(time_zone::from_utc);
        }

        if !synchronized || pps_pending() {
            continue;
        }

        let Some(centiseconds) = centiseconds_since_pps() else {
            continue;
        };

        // Keep the last 20 ms silent so the next PPS and its synchronized UART
        // start bit cannot be delayed by a MAX7221 transaction.
        if centiseconds >= DISPLAY_QUIET_AFTER_CENTISECONDS {
            continue;
        }

        if centiseconds != last_centiseconds {
            display.write_str(clock.format(&mut text, centiseconds));
            last_centiseconds = centiseconds;
        }
    }
}
