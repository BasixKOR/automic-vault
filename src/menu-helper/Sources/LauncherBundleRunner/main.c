#include <Security/Security.h>
#include <mach-o/dyld.h>
#include <signal.h>
#include <spawn.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <sys/wait.h>
#include <unistd.h>

extern char **environ;

static volatile sig_atomic_t child_pid = 0;
static volatile const char sealed_payload_hashes[513] = "AVLB_PAYLOAD_CDHASHES:";

static void forward_signal(int signal_number) {
    pid_t pid = (pid_t)child_pid;
    if (pid > 0) kill(pid, signal_number);
}

static char *copy_self_path(void) {
    uint32_t capacity = 0;
    _NSGetExecutablePath(NULL, &capacity);
    char *path = malloc(capacity);
    if (path == NULL || _NSGetExecutablePath(path, &capacity) != 0) {
        free(path);
        return NULL;
    }
    return path;
}

static CFDictionaryRef copy_signing_information(SecCodeRef code) {
    CFDictionaryRef information = NULL;
    OSStatus status = SecCodeCopySigningInformation(
        code,
        kSecCSSigningInformation,
        &information
    );
    return status == errSecSuccess ? information : NULL;
}

static bool sealed_hashes_contains(CFDataRef hash) {
    CFIndex count = CFDataGetLength(hash);
    const UInt8 *bytes = CFDataGetBytePtr(hash);
    char *hex = malloc((size_t)count * 2 + 1);
    if (hex == NULL) return false;
    for (CFIndex index = 0; index < count; index++) {
        snprintf(hex + index * 2, 3, "%02x", bytes[index]);
    }
    const volatile char *allowed = sealed_payload_hashes
        + sizeof("AVLB_PAYLOAD_CDHASHES:") - 1;
    bool found = false;
    size_t hex_length = (size_t)count * 2;
    while (*allowed != '\0') {
        size_t length = 0;
        while (allowed[length] != '\0' && allowed[length] != ',') length++;
        if (length == hex_length) {
            found = true;
            for (size_t index = 0; index < length; index++) {
                if (allowed[index] != hex[index]) {
                    found = false;
                    break;
                }
            }
            if (found) break;
        }
        if (allowed[length] == '\0') break;
        allowed += length + 1;
    }
    if (!found) fprintf(stderr, "Launcher Bundle: child hash %s was not sealed\n", hex);
    free(hex);
    return found;
}

static bool suspended_child_matches(pid_t pid) {
    CFNumberRef process_id = CFNumberCreate(NULL, kCFNumberIntType, &pid);
    if (process_id == NULL) {
        fprintf(stderr, "Launcher Bundle: child process identity is unavailable\n");
        return false;
    }
    const void *keys[] = { kSecGuestAttributePid };
    const void *values[] = { process_id };
    CFDictionaryRef attributes = CFDictionaryCreate(
        NULL,
        keys,
        values,
        1,
        &kCFTypeDictionaryKeyCallBacks,
        &kCFTypeDictionaryValueCallBacks
    );
    CFRelease(process_id);
    if (attributes == NULL) {
        fprintf(stderr, "Launcher Bundle: child attributes are unavailable\n");
        return false;
    }
    SecCodeRef code = NULL;
    OSStatus status = SecCodeCopyGuestWithAttributes(
        NULL,
        attributes,
        kSecCSDefaultFlags,
        &code
    );
    CFRelease(attributes);
    if (status != errSecSuccess) {
        fprintf(stderr, "Launcher Bundle: child lookup failed (%d)\n", (int)status);
        return false;
    }
    status = SecCodeCheckValidity(code, kSecCSDefaultFlags, NULL);
    CFDictionaryRef information = status == errSecSuccess
        ? copy_signing_information(code)
        : NULL;
    CFRelease(code);
    if (information == NULL) {
        fprintf(stderr, "Launcher Bundle: child signature check failed (%d)\n", (int)status);
        return false;
    }
    CFDataRef hash = CFDictionaryGetValue(information, kSecCodeInfoUnique);
    if (hash == NULL) fprintf(stderr, "Launcher Bundle: child code hash is unavailable\n");
    bool matches = hash != NULL && sealed_hashes_contains(hash);
    CFRelease(information);
    return matches;
}

int main(int argc, char **argv) {
    char *self_path = copy_self_path();
    if (self_path == NULL || sealed_payload_hashes[0] == '\0') {
        fprintf(stderr, "Launcher Bundle: could not verify launcher identity\n");
        free(self_path);
        return 126;
    }
    char *macos = strrchr(self_path, '/');
    if (macos == NULL) return 126;
    *macos = '\0';
    size_t payload_size = strlen(self_path) + strlen("/../Resources/payload") + 1;
    char *payload = malloc(payload_size);
    if (payload == NULL) return 126;
    snprintf(payload, payload_size, "%s/../Resources/payload", self_path);

    char **child_argv = calloc((size_t)argc + 1, sizeof(char *));
    if (child_argv == NULL) return 126;
    child_argv[0] = payload;
    for (int index = 1; index < argc; index++) child_argv[index] = argv[index];

    int forwarded[] = { SIGHUP, SIGINT, SIGQUIT, SIGTERM };
    struct sigaction action = { .sa_handler = forward_signal };
    sigemptyset(&action.sa_mask);
    for (size_t index = 0; index < sizeof(forwarded) / sizeof(forwarded[0]); index++) {
        sigaction(forwarded[index], &action, NULL);
    }

    posix_spawnattr_t attributes;
    if (posix_spawnattr_init(&attributes) != 0
        || posix_spawnattr_setflags(&attributes, POSIX_SPAWN_START_SUSPENDED) != 0) {
        fprintf(stderr, "Launcher Bundle: could not prepare payload\n");
        return 126;
    }
    pid_t pid = 0;
    int spawn_status = posix_spawn(&pid, payload, NULL, &attributes, child_argv, environ);
    posix_spawnattr_destroy(&attributes);
    if (spawn_status != 0) {
        fprintf(stderr, "Launcher Bundle: could not start payload: %s\n", strerror(spawn_status));
        return 126;
    }
    child_pid = pid;
    if (!suspended_child_matches(pid)) {
        kill(pid, SIGKILL);
        waitpid(pid, NULL, 0);
        fprintf(stderr, "Launcher Bundle: payload identity changed\n");
        return 126;
    }

    if (kill(pid, SIGCONT) != 0) {
        fprintf(stderr, "Launcher Bundle: could not resume payload: %s\n", strerror(errno));
        kill(pid, SIGKILL);
        waitpid(pid, NULL, 0);
        return 126;
    }
    int status = 0;
    while (waitpid(pid, &status, 0) < 0) {
        if (errno != EINTR) return 126;
    }
    child_pid = 0;
    free(child_argv);
    free(payload);
    free(self_path);
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) {
        fprintf(stderr, "Launcher Bundle: payload exited from signal %d\n", WTERMSIG(status));
        signal(WTERMSIG(status), SIG_DFL);
        raise(WTERMSIG(status));
        return 128 + WTERMSIG(status);
    }
    return 126;
}
