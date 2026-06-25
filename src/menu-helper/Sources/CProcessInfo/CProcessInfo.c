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

