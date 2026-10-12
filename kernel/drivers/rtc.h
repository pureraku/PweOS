#ifndef DRIVERS_RTC_H
#define DRIVERS_RTC_H

#include <stdint.h>

typedef struct {
    uint8_t second;
    uint8_t minute;
    uint8_t hour;
    uint8_t day;
    uint8_t month;
    uint32_t year;
} rtc_datetime_t;

void rtc_init(void);
void rtc_get_datetime(rtc_datetime_t *dt);

#endif // DRIVERS_RTC_H
