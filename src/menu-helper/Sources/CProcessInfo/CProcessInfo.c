#include "CProcessInfo.h"

#include <libproc.h>
#include <string.h>
#include <sys/sysctl.h>
#include <sys/socket.h>
#include <sys/un.h>

bool av_peer_pid(int fd, pid_t *pid_out) {
    socklen_t len = sizeof(*pid_out);
    return getsockopt(fd, SOL_LOCAL, LOCAL_PEERPID, pid_out, &len) == 0;
}

bool av_process_identity(pid_t pid, AVProcessIdentity *identity_out) {
    struct kinfo_proc info;
    size_t len = sizeof(info);
    int mib[] = { CTL_KERN, KERN_PROC, KERN_PROC_PID, pid };
    memset(&info, 0, sizeof(info));
    if (sysctl(mib, 4, &info, &len, NULL, 0) != 0 || len == 0) {
        return false;
    }

    memset(identity_out, 0, sizeof(*identity_out));
    identity_out->pid = pid;
    identity_out->ppid = info.kp_eproc.e_ppid;
    identity_out->start_usec =
        ((uint64_t)info.kp_proc.p_starttime.tv_sec * 1000000ULL) +
        (uint64_t)info.kp_proc.p_starttime.tv_usec;
    proc_pidpath(pid, identity_out->path, sizeof(identity_out->path));
    return true;
}

bool av_process_arguments(pid_t pid, char *out, size_t out_len) {
    if (out_len == 0) {
        return false;
    }
    out[0] = '\0';

    char buffer[8192];
    size_t len = sizeof(buffer);
    int mib[] = { CTL_KERN, KERN_PROCARGS2, pid };
    if (sysctl(mib, 3, buffer, &len, NULL, 0) != 0 || len <= sizeof(int)) {
        return false;
    }

    int argc = 0;
    memcpy(&argc, buffer, sizeof(argc));
    if (argc <= 0) {
        return false;
    }

    char *cursor = buffer + sizeof(argc);
    char *end = buffer + len;
    while (cursor < end && *cursor != '\0') cursor++;
    while (cursor < end && *cursor == '\0') cursor++;

    size_t written = 0;
    for (int i = 0; i < argc && cursor < end && written + 1 < out_len; i++) {
        size_t arg_len = strnlen(cursor, (size_t)(end - cursor));
        if (arg_len == 0) {
            break;
        }
        if (i > 0 && written + 1 < out_len) {
            out[written++] = '\n';
        }
        size_t copy_len = arg_len;
        if (copy_len > out_len - written - 1) {
            copy_len = out_len - written - 1;
        }
        memcpy(out + written, cursor, copy_len);
        written += copy_len;
        cursor += arg_len + 1;
    }
    out[written] = '\0';
    return written > 0;
}
