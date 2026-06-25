#pragma once

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <sys/types.h>

#ifndef PROC_PIDPATHINFO_MAXSIZE
#define PROC_PIDPATHINFO_MAXSIZE 4096
#endif

typedef struct {
    pid_t pid;
    pid_t ppid;
    uint64_t start_usec;
    char path[PROC_PIDPATHINFO_MAXSIZE];
} AVProcessIdentity;

bool av_peer_pid(int fd, pid_t *pid_out);
bool av_process_identity(pid_t pid, AVProcessIdentity *identity_out);
bool av_process_arguments(pid_t pid, char *out, size_t out_len);
