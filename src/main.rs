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

mod debounce;
mod time_base;

mod button;
use button::Button;

mod switch;
use switch::Switch;

mod buzzer;
use buzzer::Buzzer;

mod song;
use song::{Note, Song, SongConfig, SongStep};

mod alarm;
use alarm::Alarm;

const ALARM_SONG: [SongStep; 4] = [
    SongStep {
        note: Note::C5,
        beats: 1,
    },
    SongStep {
        note: Note::Rest,
        beats: 1,
    },
    SongStep {
        note: Note::C5,
        beats: 1,
    },
    SongStep {
        note: Note::Rest,
        beats: 1,
    },
];
const ALARM_SONG_BEATS_PER_MINUTE: u16 = 240;

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

    // Buttons are active-high with an external pull-down on the board, so a
    // floating input reads the board's pull-down when idle and full VCC
    // when pressed.
    let mut btn0 = Button::new(pins.a0.into_floating_input());
    let mut btn1 = Button::new(pins.a1.into_floating_input());
    let mut btn2 = Button::new(pins.a2.into_floating_input());
    let mut btn3 = Button::new(pins.a3.into_floating_input());
    let mut btn4 = Button::new(pins.a4.into_floating_input());
    let mut btn5 = Button::new(pins.a5.into_floating_input());

    // A6 has no digital input buffer on the ATmega328P, so the switch is
    // read through the ADC instead.
    let mut adc = arduino_hal::Adc::new(dp.ADC, Default::default());
    let mut switch0 = Switch::new();

    // Logs each successful GPS sync (date + time) for field diagnostics.
    let mut serial = arduino_hal::default_serial!(dp, pins, 57600);

    // D6 = OC0A. Ownership of the pin is only needed to claim the DDR bit;
    // Timer0's hardware compare match drives the pin from here on.
    let _buzzer_pin = pins.d6.into_output();
    let mut song = Song::new(Buzzer::new(dp.TC0));
    let mut alarm = Alarm::new();
    let mut alarm_ringing = false;
    let mut alarm_fired_this_match = false;

    let mut text: String<16> = String::new();
    let mut clock = TimeClock::new(0, 0, 0);
    let mut primed_time: Option<time_zone::LocalDateTime> = None;
    let mut last_centiseconds = u8::MAX;
    let mut synchronized = false;
    let mut initial_sync_pending = true;
    let mut serial_guard_active = false;
    let mut showing_date = false;
    let mut last_alarm_render: Option<(u8, u8, u8, bool)> = None;

    // Timer1 runs freely at 16 MHz / 256 = 62,500 Hz.  One second is 62,500
    // counts, so it never overflows while valid PPS pulses are present.
    dp.TC1.tccr1a().write(|w| unsafe { w.bits(0) });
    dp.TC1.tccr1b().write(|w| w.cs1().prescale_256());
    dp.TC1.tcnt1().write(|w| w.set(0));
    dp.TC1.tifr1().write(|w| w.tov1().set_bit());

    // PCINT2 covers port D.  PCINT23 corresponds to PD7 / Arduino D7.
    dp.EXINT.pcicr().write(|w| unsafe { w.bits(0b100) });
    dp.EXINT.pcmsk2().write(|w| w.set(1 << 7));

    // Timer2 provides a free-running millisecond tick for the button and
    // switch debounce state machines, independent of PPS sync.
    time_base::init(&dp.TC2);

    unsafe { avr_device::interrupt::enable() };

    display.write_str(clock.format(&mut text, 0));

    loop {
        let now_ms = time_base::millis();
        btn0.update(now_ms);
        btn1.update(now_ms);
        btn2.update(now_ms);
        btn3.update(now_ms);
        btn4.update(now_ms);
        btn5.update(now_ms);
        switch0.update(&mut adc, now_ms);
        song.update(now_ms);

        // Edge-detected once per iteration regardless of mode, so a press
        // that starts in one mode and releases in another is never missed.
        // Scroll/inc/dec auto-repeat on hold; BTN5's mode toggle does not.
        let btn0_edge = btn0.take_repeat_edge(now_ms);
        let btn1_edge = btn1.take_repeat_edge(now_ms);
        let btn2_edge = btn2.take_repeat_edge(now_ms);
        let btn3_edge = btn3.take_repeat_edge(now_ms);
        let btn5_edge = btn5.take_pressed_edge();

        if btn5_edge {
            if alarm.is_editing() {
                alarm.stop_editing();
                last_centiseconds = u8::MAX;
            } else {
                alarm.start_editing();
            }
        }

        if alarm.is_editing() {
            if btn3_edge {
                alarm.scroll_left();
            }
            if btn0_edge {
                alarm.scroll_right();
            }
            if btn2_edge {
                alarm.increment();
            }
            if btn1_edge {
                alarm.decrement();
            }
        }

        // The alarm only fires once GPS sync makes the clock meaningful, and
        // never while it is being edited. Turning the switch off dismisses
        // the current occurrence: alarm_fired_this_match keeps it from
        // re-triggering if the switch is flipped back on before the clock
        // moves past the matching minute.
        let matches_now = alarm.matches(clock.hours(), clock.minutes());

        if !alarm_ringing
            && !alarm_fired_this_match
            && synchronized
            && !alarm.is_editing()
            && switch0.is_high()
            && matches_now
        {
            song.configure(SongConfig {
                steps: &ALARM_SONG,
                beats_per_minute: ALARM_SONG_BEATS_PER_MINUTE,
                repeat: true,
            });
            song.enable(now_ms);
            alarm_ringing = true;
            alarm_fired_this_match = true;
        } else if alarm_ringing && !switch0.is_high() {
            song.disable();
            alarm_ringing = false;
        }

        if !matches_now {
            alarm_fired_this_match = false;
        }

        if take_pps() {
            synchronized = true;
            last_centiseconds = u8::MAX;

            // The NMEA timestamp follows this PPS.  Advance it before staging
            // for the next PPS, so that edge receives the time it represents.
            let applied_primed_time = primed_time.take();
            if let Some(local) = applied_primed_time {
                clock.set_time(local.hours, local.minutes, local.seconds);
                clock.set_date(local.year, local.month, local.day);

                for &b in clock.format_date(&mut text).as_bytes() {
                    serial.write_byte(b);
                }
                serial.write_byte(b' ');
                for &b in clock.format(&mut text, 0).as_bytes() {
                    serial.write_byte(b);
                }
                serial.write_byte(b'\r');
                serial.write_byte(b'\n');
            } else {
                clock.tick_second();
            }

            // Capture once at boot and then only at minute boundaries.  Do
            // not capture again immediately after applying a just-received
            // timestamp, even if that time happens to be :00.
            let capture_now = initial_sync_pending
                || (applied_primed_time.is_none() && clock.is_minute_boundary());
            initial_sync_pending = false;

            if capture_now {
                // No display I/O occurs before this capture: the UART start
                // bit may begin at the same instant as PPS.
                primed_time = gps_uart
                    .capture_rmc_time()
                    .map(|utc| time_zone::from_utc(utc.next_second()));
            }

            // The next PPS starts a minute-boundary capture.  Reserve its
            // final 20 ms for the synchronized UART start bit.
            serial_guard_active = clock.seconds() == 59;
        }

        // Alarm-set mode takes over the display until BTN5 confirms it.
        if alarm.is_editing() {
            let blink_on = (now_ms / 500) % 2 == 0;
            let render_state = (alarm.hours(), alarm.minutes(), alarm.cursor(), blink_on);
            if last_alarm_render != Some(render_state) {
                display.write_str(alarm.format_edit(&mut text, blink_on));
                last_alarm_render = Some(render_state);
            }
            continue;
        }
        last_alarm_render = None;

        // BTN4 held shows the date in place of the running clock. The clock
        // itself (above) keeps advancing underneath so nothing is lost.
        if btn4.is_pressed() {
            if !showing_date {
                display.write_str(clock.format_date(&mut text));
                showing_date = true;
            }
            continue;
        } else if showing_date {
            showing_date = false;
            // Force the next tick to redraw the clock immediately.
            last_centiseconds = u8::MAX;
        }

        if !synchronized || pps_pending() {
            continue;
        }

        let Some(centiseconds) = centiseconds_since_pps() else {
            continue;
        };

        if serial_guard_active && centiseconds >= DISPLAY_QUIET_AFTER_CENTISECONDS {
            continue;
        }

        if centiseconds != last_centiseconds {
            display.write_str(clock.format(&mut text, centiseconds));
            last_centiseconds = centiseconds;
        }
    }
}
