gps_clock_purple
================

Rust project for the _Arduino Nano_, driving a GPS-disciplined 8-digit
seven-segment clock with buttons, a switch, and a buzzer for an alarm.

## Build Instructions
1. Install prerequisites as described in the [`avr-hal` README] (`avr-gcc`, `avr-libc`, `avrdude`, [`ravedude`]).

2. Run `cargo build` to build the firmware.

3. Run `cargo run` to flash the firmware to a connected board.  If `ravedude`
   fails to detect your board, check its documentation at
   <https://crates.io/crates/ravedude>.

4. `ravedude` will open a console session after flashing where you can interact
   with the UART console of your board.

[`avr-hal` README]: https://github.com/Rahix/avr-hal#readme
[`ravedude`]: https://crates.io/crates/ravedude

## Hardware overview

The clock is built around an ATmega328P (Arduino Nano). Pin assignments live
in the board's `pinList.h`-equivalent and are summarized here for reference:

| Function | Pin(s) |
| --- | --- |
| Buttons BTN0-BTN5 | A0-A5 |
| Switch SW0 | A6 (analog-only pin - no digital input buffer on the ATmega328P, so it's read via the ADC) |
| Buzzer | D6 (Timer0 / OC0A) |
| 7-segment display (MAX7221, bit-banged) | D2 (data), D4 (clock), D5 (chip select) |
| GPS 1PPS | D7 (PCINT23) |
| GPS serial (soft UART, RX only) | D8 |
| Debug/diagnostic serial | D0/D1 (hardware USART, exposed through `ravedude`'s console) |

The clock is fully disciplined by GPS: the 1PPS line marks each second's
boundary in hardware, and an NMEA RMC sentence read once at boot and then
once per minute supplies the wall-clock date and time. Between GPS captures,
the display free-runs from Timer1 (reset every PPS pulse) for sub-second
precision.

## Operating modes

The display is normally showing the running clock as `HH.MM.SS.CC` (hours,
minutes, seconds, centiseconds). Several button combinations temporarily (or
permanently, for settings) take over the display. Where two modes could both
claim the display at once, priority is: UTC-offset selection, then alarm
setting, then the date view, then the normal clock.

### Running clock (default)
`HH.MM.SS.CC`, updated continuously once GPS has synchronized. Before the
first successful sync the display holds at `00.00.00.00`.

### BTN4 (hold): show the date
Holding BTN4 replaces the clock with the date as `MMDD` (month then day,
each zero-padded to two digits) for as long as it's held; releasing returns
to the clock immediately. The clock keeps advancing underneath while the
date is shown, so nothing is lost.

The year is intentionally **not** shown here. GPS date parsing has an
unresolved, intermittent issue where the year field is occasionally wrong
(typically once, near boot) even though month, day, and time have always
been reliable - see "Known issue: occasional bad GPS year" below. The full
parsed date, including year, is still written to the diagnostic serial log
every time a GPS fix is applied, in case this is worth revisiting later.

### BTN5 (press): set the alarm
Pressing BTN5 enters alarm-set mode, showing the alarm's `HH.MM` with the
selected field blinking (both digits of that field blink together, not a
single digit):

- **BTN3** / **BTN0** - move the selected field left (hour) / right (minute).
- **BTN2** / **BTN1** - increment / decrement the selected field by one,
  wrapping (hour 0-23, minute 0-59). Both auto-repeat: an initial press
  fires once, then holding repeats after a short delay.
- **BTN5** again - locks in the alarm time and returns to the clock.

The alarm is not persisted across power cycles; it resets to `00:00` on
each boot.

### SW0: arm / silence the alarm
With SW0 high and the clock GPS-synchronized, the alarm fires the moment
the clock's hour and minute match the set alarm time, playing a short
repeating beep pattern. Turning SW0 off silences it immediately. If SW0 is
turned back on within the same matching minute, the alarm does **not**
immediately re-trigger - it stays dismissed until the clock moves to a
different minute, so toggling the switch acts as a real "off," not a
momentary mute.

### BTN4+BTN5 (hold together): set the UTC offset
Holding BTN4 and BTN5 down together enters UTC-offset selection, showing
`UTC  N05`-style output: `UTC`, two blank digits, a sign letter (`N` for
negative, `P` for positive - clearer at a glance on this display than a
single-segment `+`/`-`), then the two-digit offset magnitude in hours.

- **BTN2** / **BTN1** - step the offset up / down by one hour, wrapping at
  the ends of the UTC-12 .. UTC+14 range (stepping past +14 wraps to -12,
  and past -12 wraps to +14, matching how those two extremes are actually
  adjacent going the other way around the globe).
- **BTN4 alone**, **BTN5 alone**, or the **BTN4+BTN5 combo again** - any of
  the three locks in the current selection and exits.

There is no automatic daylight-saving adjustment: DST rules (whether a
region observes it, and exactly when it changes) vary too much worldwide to
infer from a UTC offset alone. When DST starts or ends locally, nudge the
offset by one hour (two BTN1/BTN2 presses) the same way you'd set it
initially.

Exiting applies the new offset to the currently displayed time immediately
- it does not wait for the next GPS capture, so the clock never sits
displaying the wrong local time after a change.

The offset resets to `UTC+0` on each boot; it is not persisted.

### Diagnostic serial log
The hardware USART (D0/D1, 57600 baud) logs one line per successful GPS
capture with the full parsed date and time, including the year that the
physical display omits:
```
MM.DD.YY HH.MM.SS
```
This is visible in the console `cargo run` opens after flashing.

## Known issue: occasional bad GPS year

The GPS-parsed year is, rarely, wrong - while month, day, and time have
been reliable every time it's been checked. This was investigated
extensively (raw NMEA sentence bytes were confirmed correct on the wire;
every parsing/transform function was independently verified correct by
hand; several different software-UART receiver implementations, including
one with zero interrupt-jitter exposure, all reproduced the identical wrong
value) without finding the root cause. It appears tied to the first GPS
capture after power-up.

Given it's benign (self-corrects on the very next per-minute capture) and
the root cause wasn't found despite substantial effort, the pragmatic fix
was to stop displaying the year on the physical display rather than keep
chasing it - see the "Drop year from the date display" commit for the full
write-up. The diagnostic serial log still records every parsed year, in
case a future investigation (e.g. with a hardware debugger, rather than
print-based debugging) wants the data.

## Codebase organization

All source lives in `src/`. `main.rs` owns every hardware peripheral and
the main loop; everything else is a focused driver or a piece of pure
domain logic that `main.rs` wires together.

| Module | Responsibility |
| --- | --- |
| `main.rs` | Entry point. Owns all peripherals, the PPS interrupt, and the main loop: polls every input once per iteration, advances the clock/alarm/song state, and dispatches the display to whichever mode currently has priority. |
| `nmea.rs` | `UtcDateTime` (the GPS-parsed UTC date/time) and NMEA RMC sentence parsing: checksum validation, field extraction, and `next_second()`/`next_day()`/`previous_day()` calendar rollover math. |
| `soft_uart.rs` | Bit-banged software UART receiver for the GPS module's serial line (D8), used because the hardware USART is reserved for the diagnostic console on D0/D1. |
| `time_zone.rs` | `UtcOffset` (the worldwide, user-selectable UTC offset: step, wrap, format) and `from_utc()`, which converts a `UtcDateTime` to local `LocalDateTime` under a given offset, including day rollover in either direction. |
| `time_clock.rs` | `TimeClock`, the running display clock's own state (time + date). Advances one second at a time between GPS captures, accepts a fresh time/date from a capture, and can shift by a signed number of hours (used when the UTC offset changes) without waiting for a new capture. |
| `max7221.rs` | Driver for the MAX7221 seven-segment display controller: bit-banged SPI-like protocol plus the custom decoder (`decode_char`) mapping ASCII characters to seven-segment patterns. |
| `debounce.rs` | The generic "stable for N ms" debounce state machine shared by `Button` and `Switch`. |
| `button.rs` | `Button`: a debounced digital input with both a single-press edge and a press-and-hold auto-repeat edge (initial delay, then a steady repeat rate). |
| `switch.rs` | `Switch`: a debounced two-state input read through the ADC, since SW0's pin (A6) has no digital input buffer on this chip. |
| `time_base.rs` | A free-running millisecond tick (Timer2), independent of GPS PPS sync, that debounce and button-repeat timing are built on. |
| `buzzer.rs` | `Buzzer`: a hardware-PWM tone generator (Timer0/OC0A) with configure/enable/disable, run-to-completion semantics. |
| `song.rs` | `Song`: owns a `Buzzer` and sequences a list of notes and rests at a given tempo, with optional looping - used to play the alarm's beep pattern. |
| `alarm.rs` | `Alarm`: the settable alarm time and its digit-editing cursor/state, independent of whether it's currently armed (that's `SW0`) or ringing (that's driven from `main.rs`). |

### Main loop shape

Every iteration of `main.rs`'s loop:
1. Polls and debounces every button and the switch, and advances the song
   player.
2. Handles button-combo/mode-toggle edges (UTC-offset selection, alarm
   editing) and applies any pending edits.
3. Checks whether the alarm should start or stop ringing.
4. If a PPS interrupt fired, advances or resyncs the clock and, on capture
   cycles, reads a fresh GPS sentence.
5. Dispatches the display to whichever mode has priority, falling through
   to the normal running clock if none of them claim it.

Each mode module (`Alarm`, and the UTC-offset selection state that lives
directly in `main.rs`) only owns its own settings and editing state; `main.rs`
is what decides display priority and wires inputs to the right mode.

## License
Licensed under either of

 - Apache License, Version 2.0
   ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
 - MIT license
   ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contribution
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall
be dual licensed as above, without any additional terms or conditions.
