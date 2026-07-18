/// Fixed offset from GPS/NMEA UTC, in whole hours.
///
/// Eastern Daylight Time (EDT) is UTC-4.  Change this manually to UTC-5 when
/// Eastern Standard Time (EST) resumes; this firmware intentionally does not
/// calculate daylight-saving transitions.
pub const UTC_OFFSET_HOURS: i8 = -4;

/*
UTC offset reference (use standard-time offsets unless your location is
currently observing daylight-saving time):

 UTC-12: Baker Island, Howland Island
 UTC-11: American Samoa, Niue, Midway Atoll
 UTC-10: Hawaii, Tahiti, Cook Islands
 UTC-9:  Alaska (most areas, standard time)
 UTC-8:  Pacific Time — Los Angeles, Seattle, Vancouver
 UTC-7:  Mountain Time — Denver, Phoenix*, Calgary
 UTC-6:  Central Time — Chicago, Mexico City, Costa Rica
 UTC-5:  Eastern Standard Time — New York, Toronto, Lima, Bogotá
 UTC-4:  Eastern Daylight Time — New York/Toronto in summer; Halifax,
          Puerto Rico, Barbados
 UTC-3:  Buenos Aires, São Paulo, Greenland (some areas)
 UTC-2:  South Georgia, parts of Greenland
 UTC-1:  Azores, Cape Verde
 UTC+0:  UTC/GMT — London (winter), Reykjavík, Lisbon (winter), Accra
 UTC+1:  Central European Time — Paris, Berlin, Rome, Lagos
 UTC+2:  Eastern European Time — Athens, Helsinki, Cairo, Johannesburg
 UTC+3:  Moscow, Istanbul, Riyadh, Nairobi
 UTC+4:  Dubai, Abu Dhabi, Baku, Muscat
 UTC+5:  Pakistan, Maldives, Tashkent
 UTC+5:30: India, Sri Lanka (not supported by this whole-hour setting)
 UTC+6:  Bangladesh, Bhutan, Almaty
 UTC+7:  Bangkok, Jakarta, Ho Chi Minh City
 UTC+8:  China, Singapore, Perth, Taipei, Hong Kong
 UTC+9:  Japan, South Korea
 UTC+10: Brisbane, Papua New Guinea, Vladivostok
 UTC+11: Solomon Islands, New Caledonia
 UTC+12: Fiji, New Zealand (winter), Marshall Islands
 UTC+13: Samoa, Tonga, New Zealand (summer)
 UTC+14: Kiritimati (Line Islands)

* Phoenix stays on UTC-7 year-round.  Several regions use 30- or 45-minute
offsets; those require extending this module beyond the current whole-hour
configuration.
*/

/// Convert an NMEA UTC time to the configured local time, wrapping at midnight.
pub fn from_utc(hours: u8, minutes: u8, seconds: u8) -> (u8, u8, u8) {
    let local_hours = (hours as i8 + UTC_OFFSET_HOURS).rem_euclid(24) as u8;
    (local_hours, minutes, seconds)
}
