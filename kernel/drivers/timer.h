#ifndef DRIVERS_TIMER_H
#define DRIVERS_TIMER_H

#include <stdint.h>

#define PIT_DEFAULT_HZ 100 // 100 Hz = 10 ms per tick

void timer_init(uint32_t frequency);
uint32_t timer_get_ticks(void);
uint32_t timer_get_uptime_seconds(void);
void timer_sleep(uint32_t ticks);

#endif // DRIVERS_TIMER_H
