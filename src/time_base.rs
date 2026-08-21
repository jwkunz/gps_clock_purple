use arduino_hal::pac::TC2;
use avr_device::interrupt::Mutex;
use core::cell::Cell;

/// Timer2 runs at 16 MHz / 64 = 250 kHz and is reset every 250 counts by CTC
/// mode, so each compare match marks exactly one millisecond. This is a
/// free-running time base independent of Timer1, which only advances
/// relative to the GPS PPS edge and is unavailable before the first sync.
const COMPARE_TOP: u8 = 249;

static MILLIS: Mutex<Cell<u32>> = Mutex::new(Cell::new(0));

/// Configure Timer2 as the free-running millisecond tick. Call once at
/// startup before enabling interrupts.
pub fn init(tc2: &TC2) {
    tc2.tccr2a().write(|w| w.wgm2().ctc());
    tc2.ocr2a().write(|w| w.set(COMPARE_TOP));
    tc2.tcnt2().write(|w| w.set(0));
    tc2.timsk2().write(|w| w.ocie2a().set_bit());
    tc2.tccr2b().write(|w| w.cs2().prescale_64());
}

#[avr_device::interrupt(atmega328p)]
fn TIMER2_COMPA() {
    avr_device::interrupt::free(|cs| {
        let cell = MILLIS.borrow(cs);
        cell.set(cell.get().wrapping_add(1));
    });
}

/// Milliseconds elapsed since `init`, wrapping at `u32::MAX` (about 49 days).
pub fn millis() -> u32 {
    avr_device::interrupt::free(|cs| MILLIS.borrow(cs).get())
}
