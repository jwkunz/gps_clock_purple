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

pub(crate) fn pps_pending() -> bool {
    avr_device::interrupt::free(|cs| PPS_PENDING.borrow(cs).get())
}

fn take_pps() -> bool {
    avr_device::interrupt::free(|cs| PPS_PENDING.borrow(cs).replace(false))
}

#[avr_device::interrupt(atmega328p)]
fn PCINT2() {
    // Pin-change interrupts fire on both PPS edges.  D7 is PD7; only retain
    // the active-high edge.  Direct register access keeps the ISR tiny.
    let port_d = unsafe { arduino_hal::pac::PORTD::steal().pind().read().bits() };
    if port_d & (1 << 7) != 0 {
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

    // PCINT2 covers port D.  PCINT23 corresponds to PD7 / Arduino D7.
    dp.EXINT.pcicr().write(|w| unsafe { w.bits(0b100) });
    dp.EXINT.pcmsk2().write(|w| w.set(1 << 7));
    unsafe { avr_device::interrupt::enable() };

    display.write_str(clock.format(&mut text));

    loop {
        if !take_pps() {
            continue;
        }

        // An RMC sentence received after this edge is intentionally applied at
        // the *next* edge.  This accepts the GPS module's known one-second
        // NMEA lag while keeping all displayed second changes PPS-aligned.
        if let Some((hours, minutes, seconds)) = primed_time.take() {
            clock.set_time(hours, minutes, seconds);
        } else {
            clock.tick_second();
        }

        // Capture before doing display I/O: the first UART start bit may begin
        // immediately with PPS.  The ISR only records a possible next pulse.
        primed_time = gps_uart
            .capture_rmc_time()
            .map(|(hours, minutes, seconds)| time_zone::from_utc(hours, minutes, seconds));

        // If capture ended because a new PPS arrived, return to the top of the
        // loop immediately.  A MAX7221 transaction here could otherwise make
        // us late for that new sentence's first UART start bit.
        if !pps_pending() {
            display.write_str(clock.format(&mut text));
        }
    }
}
