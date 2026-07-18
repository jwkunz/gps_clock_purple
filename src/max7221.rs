use embedded_hal::digital::OutputPin;

fn decode_char(c:char) -> u8 {

    match c {

        '0' => 0x7E,
        '1' => 0x30,
        '2' => 0x6D,
        '3' => 0x79,
        '4' => 0x33,
        '5' => 0x5B,
        '6' => 0x5F,
        '7' => 0x70,
        '8' => 0x7F,
        '9' => 0x73,

        '-' => 0x01,

        'A' => 0x77,
        'B' => 0x7F,
        'C' => 0x4E,
        'D' => 0x7E,
        'E' => 0x4F,
        'F' => 0x47,
        'G' => 0x5F,
        'H' => 0x37,
        'I' => 0x06,
        'J' => 0x3C,
        'L' => 0x0E,
        'P' => 0x67,

        '!' => 0xA0,

        _ => 0x00,
    }
}


const MX_NUM_SEGMENTS: u8 = 8;

const MX_ADDR_INTENSITY: u8 = 0x0A;
const MX_ADDR_SHUTDOWN: u8 = 0x0C;
const MX_ADDR_DISPTEST: u8 = 0x0F;
const MX_ADDR_DECODE_MODE: u8 = 0x09;
const MX_ADDR_SCANLIM: u8 = 0x0B;

const MX_DATA_OFF: u8 = 0x00;
const MX_DATA_ON: u8 = 0x01;

const MX_DATA_NO_DECODE: u8 = 0x00;

const MX_DATA_DISPTEST_NORMAL: u8 = 0x00;
const MX_DATA_DISPTEST_MODE: u8 = 0x01;

pub struct Max7221<DATA, CLK, CS>
where
    DATA: OutputPin,
    CLK: OutputPin,
    CS: OutputPin,
{
    data: DATA,
    clk: CLK,
    cs: CS,
}


impl<DATA, CLK, CS> Max7221<DATA, CLK, CS>
where
    DATA: OutputPin,
    CLK: OutputPin,
    CS: OutputPin,
{
    pub fn new(data: DATA, clk: CLK, cs: CS) -> Self {
        Self {
            data,
            clk,
            cs,
        }
    }


    fn send_data(&mut self, data: u16) {

        let _ = self.clk.set_low();
        let _ = self.cs.set_low();


        for bit in (0..16).rev() {

            let value = (data >> bit) & 1;

            if value == 1 {
                let _ = self.data.set_high();
            } else {
                let _ = self.data.set_low();
            }

            let _ = self.clk.set_high();
            let _ = self.clk.set_low();
        }


        let _ = self.cs.set_high();
    }



    fn form_code(address: u8, data: u8) -> u16 {
        ((address as u16) << 8) | data as u16
    }



    pub fn power_switch(&mut self, state: bool) {

        let value = if state {
            MX_DATA_ON
        } else {
            MX_DATA_OFF
        };

        self.send_data(
            Self::form_code(MX_ADDR_SHUTDOWN,value)
        );
    }



    pub fn set_brightness(&mut self, brightness:u8) {

        let level = brightness & 0x0F;

        self.send_data(
            Self::form_code(
                MX_ADDR_INTENSITY,
                level
            )
        );
    }



    pub fn set_number_segments(&mut self, segments:u8) {

        self.send_data(
            Self::form_code(
                MX_ADDR_SCANLIM,
                segments
            )
        );
    }



    pub fn no_decode(&mut self) {

        self.send_data(
            Self::form_code(
                MX_ADDR_DECODE_MODE,
                MX_DATA_NO_DECODE
            )
        );
    }



    pub fn display_test(&mut self, enabled:bool) {

        let value =
            if enabled {
                MX_DATA_DISPTEST_MODE
            } else {
                MX_DATA_DISPTEST_NORMAL
            };


        self.send_data(
            Self::form_code(
                MX_ADDR_DISPTEST,
                value
            )
        );
    }



    pub fn init(&mut self) {

        self.no_decode();

        self.set_number_segments(
            MX_NUM_SEGMENTS-1
        );

        self.set_brightness(1);

        self.write_str("8.8.8.8.8.8.8.8.");

        self.display_test(false);

        self.power_switch(true);
    }

    fn write_digit(&mut self, digit: u8, code: u8) {
        self.send_data(Self::form_code(digit + 1, code));
    }


    pub fn write_str(&mut self, text: &str) {
        let bytes = text.as_bytes();

        let mut digit = 0;
        let mut i = 0;

        while i < bytes.len() && digit < MX_NUM_SEGMENTS {
            let mut code = decode_char(bytes[i] as char);

            if i + 1 < bytes.len() && bytes[i + 1] == b'.' {
                code |= 0x80;
                i += 1;
            }

            self.write_digit(digit, code);

            digit += 1;
            i += 1;
        }

        while digit < MX_NUM_SEGMENTS {
            self.write_digit(digit, decode_char(' '));
            digit += 1;
        }
    }

}
