use heapless::String;

pub fn parse_time(input: &str) -> Option<(u8, u8, u8)> {

    if input.len() != 8 {
        return None;
    }

    let bytes = input.as_bytes();

    if bytes[2] != b':' || bytes[5] != b':' {
        return None;
    }

    let hours = digits(bytes[0], bytes[1])?;
    let minutes = digits(bytes[3], bytes[4])?;
    let seconds = digits(bytes[6], bytes[7])?;

    if hours > 23 {
        return None;
    }

    if minutes > 59 || seconds > 59 {
        return None;
    }

    Some((hours, minutes, seconds))
}


fn digits(a: u8, b: u8) -> Option<u8> {

    if !a.is_ascii_digit() || !b.is_ascii_digit() {
        return None;
    }

    Some(
        (a - b'0') * 10 +
        (b - b'0')
    )
}