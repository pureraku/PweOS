#ifndef PWEOS_SYS_TIME_H
#define PWEOS_SYS_TIME_H

struct timeval {
    long tv_sec;
    long tv_usec;
};

int gettimeofday(struct timeval *timeval, void *timezone);

#endif
