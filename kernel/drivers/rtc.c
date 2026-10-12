#include "drivers/rtc.h"
#include "arch/io.h"

#define CMOS_ADDRESS 0x70
#define CMOS_DATA    0x71

static uint8_t cmos_read(uint8_t reg) {
    outb(CMOS_ADDRESS, reg);
    return inb(CMOS_DATA);
}

static int rtc_is_updating(void) {
    outb(CMOS_ADDRESS, 0x0A);
    return (inb(CMOS_DATA) & 0x80);
}

static uint8_t bcd_to_binary(uint8_t bcd) {
    return ((bcd >> 4) * 10) + (bcd & 0x0F);
}

void rtc_init(void) {
    // Nothing required to initialize passive CMOS RTC
}

void rtc_get_datetime(rtc_datetime_t *dt) {
    if (!dt) return;

    // Wait until RTC is not updating
    while (rtc_is_updating());

    uint8_t second  = cmos_read(0x00);
    uint8_t minute  = cmos_read(0x02);
    uint8_t hour    = cmos_read(0x04);
    uint8_t day     = cmos_read(0x07);
    uint8_t month   = cmos_read(0x08);
    uint8_t year    = cmos_read(0x09);
    uint8_t century = cmos_read(0x32);
    uint8_t reg_b   = cmos_read(0x0B);

    // Read again to ensure consistency
    uint8_t last_second;
    do {
        last_second = second;
        while (rtc_is_updating());
        second  = cmos_read(0x00);
        minute  = cmos_read(0x02);
        hour    = cmos_read(0x04);
        day     = cmos_read(0x07);
        month   = cmos_read(0x08);
        year    = cmos_read(0x09);
        century = cmos_read(0x32);
    } while (last_second != second);

    // Convert BCD to binary if needed (bit 2 of Register B: 0 = BCD, 1 = binary)
    if (!(reg_b & 0x04)) {
        second  = bcd_to_binary(second);
        minute  = bcd_to_binary(minute);
        hour    = ((hour & 0x7F) == hour) ? bcd_to_binary(hour) : (bcd_to_binary(hour & 0x7F) | 0x80);
        day     = bcd_to_binary(day);
        month   = bcd_to_binary(month);
        year    = bcd_to_binary(year);
        if (century != 0) {
            century = bcd_to_binary(century);
        }
    }

    // Convert 12 hour to 24 hour if necessary (bit 1 of Register B: 0 = 12h, 1 = 24h)
    if (!(reg_b & 0x02) && (hour & 0x80)) {
        hour = ((hour & 0x7F) + 12) % 24;
    }

    // Compute 4-digit year
    uint32_t full_year;
    if (century != 0) {
        full_year = (uint32_t)century * 100 + year;
    } else {
        full_year = 2000 + year;
    }

    dt->second = second;
    dt->minute = minute;
    dt->hour   = hour;
    dt->day    = day;
    dt->month  = month;
    dt->year   = full_year;
}
