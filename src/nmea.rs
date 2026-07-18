/// UTC clock fields provided by an active RMC sentence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct UtcDateTime {
    pub year: u8,
    pub month: u8,
    pub day: u8,
    pub hours: u8,
    pub minutes: u8,
    pub seconds: u8,
}

/// Extract UTC date and time from a checksum-valid, active RMC sentence.
///
/// Both GPS (`GPRMC`) and multi-constellation (`GNRMC`) talkers are accepted.
pub fn parse_rmc_time(sentence: &[u8]) -> Option<UtcDateTime> {
    if !checksum_is_valid(sentence) {
        return None;
    }

    let body = &sentence[1..sentence.iter().position(|&byte| byte == b'*')?];
    let mut fields = body.split(|&byte| byte == b',');

    match fields.next()? {
        b"GPRMC" | b"GNRMC" => {}
        _ => return None,
    }

    let time = fields.next()?;
    if fields.next()? != b"A" {
        return None;
    }

    // RMC fields after status are latitude, N/S, longitude, E/W, speed,
    // course, then date (DDMMYY).
    for _ in 0..6 {
        fields.next()?;
    }
    parse_date_time(time, fields.next()?)
}

fn checksum_is_valid(sentence: &[u8]) -> bool {
    if sentence.len() < 7 || sentence[0] != b'$' {
        return false;
    }

    let Some(star) = sentence.iter().position(|&byte| byte == b'*') else {
        return false;
    };
    if star + 3 != sentence.len() {
        return false;
    }

    let Some(expected) = hex_byte(sentence[star + 1], sentence[star + 2]) else {
        return false;
    };
    let actual = sentence[1..star].iter().fold(0u8, |sum, &byte| sum ^ byte);
    actual == expected
}

fn parse_date_time(time: &[u8], date: &[u8]) -> Option<UtcDateTime> {
    // Fractional seconds, if sent, follow the first six digits.
    if time.len() < 6 || date.len() != 6 {
        return None;
    }

    let hours = two_digits(time[0], time[1])?;
    let minutes = two_digits(time[2], time[3])?;
    let seconds = two_digits(time[4], time[5])?;

    let day = two_digits(date[0], date[1])?;
    let month = two_digits(date[2], date[3])?;
    let year = two_digits(date[4], date[5])?;

    (hours < 24 && minutes < 60 && seconds < 60 && is_valid_date(day, month, year)).then_some(
        UtcDateTime {
            year,
            month,
            day,
            hours,
            minutes,
            seconds,
        },
    )
}

fn is_valid_date(day: u8, month: u8, year: u8) -> bool {
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if year % 4 == 0 => 29,
        2 => 28,
        _ => return false,
    };
    day > 0 && day <= days
}

fn two_digits(tens: u8, ones: u8) -> Option<u8> {
    if !tens.is_ascii_digit() || !ones.is_ascii_digit() {
        return None;
    }

    Some((tens - b'0') * 10 + (ones - b'0'))
}

fn hex_byte(high: u8, low: u8) -> Option<u8> {
    Some((hex_digit(high)? << 4) | hex_digit(low)?)
}

fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::parse_rmc_time;

    #[test]
    fn parses_active_gps_rmc() {
        assert_eq!(
            parse_rmc_time(
                b"$GPRMC,123519,A,4807.038,N,01131.000,E,022.4,084.4,230394,003.1,W*6A"
            ),
            Some(UtcDateTime {
                year: 94,
                month: 3,
                day: 23,
                hours: 12,
                minutes: 35,
                seconds: 19,
            })
        );
    }

    #[test]
    fn rejects_invalid_checksum_and_void_fix() {
        assert_eq!(parse_rmc_time(b"$GPRMC,123519,A,4807.038,N*00"), None);
    }
}
