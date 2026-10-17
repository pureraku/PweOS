#include "sys/time.h"
#include "pweos.h"

int gettimeofday(struct timeval *timeval, void *timezone) {
    if (!timeval) {
        errno = EINVAL;
        return -1;
    }
    return pwe_gettimeofday(timeval, timezone);
}
