use embedded_hal::digital::InputPin;

use crate::nmea;

const BIT_US: u32 = 104; // 1 / 9600 baud, rounded to the nearest microsecond.
const HALF_BIT_US: u32 = 52;
const RMC_CAPTURE_TIMEOUT_US: u32 = 750_000;
const LINE_CAPACITY: usize = 96;

/// Receive 8-N-1 UART bytes from an idle-high input using busy-loop delays.
pub struct SoftUart<RX> {
    rx: RX,
}

impl<RX> SoftUart<RX>
where
    RX: InputPin,
{
    pub fn new(rx: RX) -> Self {
        Self { rx }
    }

    /// Capture NMEA sent after a PPS edge.  A subsequent PPS aborts the old
    /// capture so a delayed sentence can never be associated with a new edge.
    pub fn capture_rmc_time(&mut self) -> Option<nmea::UtcDateTime> {
        let mut line = [0u8; LINE_CAPACITY];
        let mut line_len = 0;
        let mut remaining_us = RMC_CAPTURE_TIMEOUT_US;

        while remaining_us > 0 && !crate::pps_pending() {
            let byte = self.read_byte(&mut remaining_us)?;

            match byte {
                b'\n' => {
                    if let Some(time) = nmea::parse_rmc_time(&line[..line_len]) {
                        return Some(time);
                    }
                    line_len = 0;
                }
                b'\r' => {}
                b'$' => {
                    line[0] = byte;
                    line_len = 1;
                }
                _ if line_len < LINE_CAPACITY => {
                    line[line_len] = byte;
                    line_len += 1;
                }
                _ => line_len = 0,
            }
        }

        None
    }

    fn read_byte(&mut self, remaining_us: &mut u32) -> Option<u8> {
        // Wait for a low start bit in short increments, permitting a bounded
        // capture window and prompt abort when the next PPS arrives.
        while self.rx.is_high().ok()? {
            if *remaining_us < 8 || crate::pps_pending() {
                return None;
            }
            arduino_hal::delay_us(8);
            *remaining_us -= 8;
        }

        if *remaining_us < BIT_US * 10 {
            return None;
        }

        // Start low was observed; sample the centre of each data bit.
        arduino_hal::delay_us(BIT_US + HALF_BIT_US);
        let mut value = 0u8;
        for bit in 0..8 {
            if self.rx.is_high().ok()? {
                value |= 1 << bit;
            }
            arduino_hal::delay_us(BIT_US);
        }
        *remaining_us -= BIT_US * 10;

        // A high stop bit rejects framing errors and false start bits.
        self.rx.is_high().ok()?.then_some(value)
    }
}
