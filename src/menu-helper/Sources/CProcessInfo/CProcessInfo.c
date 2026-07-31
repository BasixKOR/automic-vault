#include "CProcessInfo.h"

#include <libproc.h>
#include <string.h>
#include <sys/sysctl.h>
#include <sys/socket.h>
#include <sys/un.h>
#include <unistd.h>

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
    identity_out->sid = getsid(pid);
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
    for (int i = 0; i < argc; i++) {
        if (cursor >= end) {
            return false;
        }
        size_t arg_len = strnlen(cursor, (size_t)(end - cursor));
        if (cursor + arg_len >= end) {
            return false;
        }
        if (memchr(cursor, '\n', arg_len) != NULL) {
            return false;
        }
        size_t required = arg_len + (i > 0 ? 1 : 0);
        if (required > out_len - written - 1) {
            return false;
        }
        if (i > 0) {
            out[written++] = '\n';
        }
        memcpy(out + written, cursor, arg_len);
        written += arg_len;
        cursor += arg_len + 1;
    }
    out[written] = '\0';
    return written > 0;
}

bool av_process_cwd(pid_t pid, char *out, size_t out_len) {
    if (out_len == 0) {
        return false;
    }
    struct proc_vnodepathinfo info;
    memset(&info, 0, sizeof(info));
    int size = proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, 0, &info, sizeof(info));
    if (size != sizeof(info) || info.pvi_cdir.vip_path[0] == '\0') {
        return false;
    }
    size_t len = strnlen(info.pvi_cdir.vip_path, sizeof(info.pvi_cdir.vip_path));
    if (len >= out_len) {
        return false;
    }
    memcpy(out, info.pvi_cdir.vip_path, len + 1);
    return true;
}
