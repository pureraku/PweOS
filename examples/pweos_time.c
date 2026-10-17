#include "sys/time.h"
#include "time.h"
#include "pweos.h"

static struct tm broken_down_time;

int gettimeofday(struct timeval *timeval, void *timezone) {
    if (!timeval) {
        errno = EINVAL;
        return -1;
    }
    return pwe_gettimeofday(timeval, timezone);
}

time_t time(time_t *result) {
    struct timeval now;
    if (gettimeofday(&now, (void *)0) < 0) return (time_t)-1;
    if (result) *result = (time_t)now.tv_sec;
    return (time_t)now.tv_sec;
}

struct tm *gmtime(const time_t *timer) {
    if (!timer || *timer < 0) return (struct tm *)0;

    long seconds = *timer;
    long days = seconds / 86400;
    seconds %= 86400;
    broken_down_time.tm_hour = (int)(seconds / 3600);
    seconds %= 3600;
    broken_down_time.tm_min = (int)(seconds / 60);
    broken_down_time.tm_sec = (int)(seconds % 60);
    broken_down_time.tm_wday = (int)((days + 4) % 7);

    int year = 1970;
    while (1) {
        int leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        int year_days = leap ? 366 : 365;
        if (days < year_days) break;
        days -= year_days;
        year++;
    }
    broken_down_time.tm_year = year - 1900;
    broken_down_time.tm_yday = (int)days;

    static const int month_days[12] = {31, 28, 31, 30, 31, 30,
                                       31, 31, 30, 31, 30, 31};
    int month = 0;
    while (month < 11) {
        int length = month_days[month];
        if (month == 1 && year % 4 == 0
            && (year % 100 != 0 || year % 400 == 0)) length++;
        if (days < length) break;
        days -= length;
        month++;
    }
    broken_down_time.tm_mon = month;
    broken_down_time.tm_mday = (int)days + 1;
    broken_down_time.tm_isdst = 0;
    return &broken_down_time;
}

struct tm *localtime(const time_t *timer) {
    /* The RTC has no timezone setting yet, so local time matches UTC. */
    return gmtime(timer);
}
