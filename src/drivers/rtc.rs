use crate::arch::io::{inb, outb};

pub const CMOS_ADDRESS: u16 = 0x70;
pub const CMOS_DATA: u16 = 0x71;

#[derive(Debug, Copy, Clone)]
pub struct RtcDateTime {
    pub second: u8,
    pub minute: u8,
    pub hour: u8,
    pub day: u8,
    pub month: u8,
    pub year: u32,
}

fn cmos_read(reg: u8) -> u8 {
    unsafe {
        outb(CMOS_ADDRESS, reg);
        inb(CMOS_DATA)
    }
}

fn rtc_is_updating() -> bool {
    unsafe {
        outb(CMOS_ADDRESS, 0x0A);
        (inb(CMOS_DATA) & 0x80) != 0
    }
}

fn bcd_to_binary(bcd: u8) -> u8 {
    ((bcd >> 4) * 10) + (bcd & 0x0F)
}

pub fn rtc_init() {
    // Passive CMOS RTC
}

pub fn rtc_get_datetime() -> RtcDateTime {
    // Wait until RTC is not updating
    while rtc_is_updating() {}

    let mut second = cmos_read(0x00);
    let mut minute;
    let mut hour;
    let mut day;
    let mut month;
    let mut year;
    let mut century;
    let reg_b = cmos_read(0x0B);

    // Read again until consistent
    let mut last_second;
    loop {
        last_second = second;
        while rtc_is_updating() {}
        second = cmos_read(0x00);
        minute = cmos_read(0x02);
        hour = cmos_read(0x04);
        day = cmos_read(0x07);
        month = cmos_read(0x08);
        year = cmos_read(0x09);
        century = cmos_read(0x32);
        if last_second == second {
            break;
        }
    }

    // Convert BCD to binary if needed (bit 2 of Register B: 0 = BCD, 1 = binary)
    if (reg_b & 0x04) == 0 {
        second = bcd_to_binary(second);
        minute = bcd_to_binary(minute);
        hour = if (hour & 0x7F) == hour {
            bcd_to_binary(hour)
        } else {
            bcd_to_binary(hour & 0x7F) | 0x80
        };
        day = bcd_to_binary(day);
        month = bcd_to_binary(month);
        year = bcd_to_binary(year);
        if century != 0 {
            century = bcd_to_binary(century);
        }
    }

    // Convert 12 hour to 24 hour if necessary (bit 1 of Register B: 0 = 12h, 1 = 24h)
    if (reg_b & 0x02) == 0 && (hour & 0x80) != 0 {
        hour = ((hour & 0x7F) + 12) % 24;
    }

    // Compute 4-digit year
    let full_year = if century != 0 {
        (century as u32) * 100 + (year as u32)
    } else {
        2000 + (year as u32)
    };

    RtcDateTime {
        second,
        minute,
        hour,
        day,
        month,
        year: full_year,
    }
}
