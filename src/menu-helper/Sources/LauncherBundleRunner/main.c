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

static char *copy_self_identifier(void) {
    SecCodeRef code = NULL;
    if (SecCodeCopySelf(kSecCSDefaultFlags, &code) != errSecSuccess) return NULL;
    CFDictionaryRef information = copy_signing_information(code);
    CFRelease(code);
    if (information == NULL) return NULL;
    CFStringRef identifier = CFDictionaryGetValue(information, kSecCodeInfoIdentifier);
    char *result = NULL;
    if (identifier != NULL) {
        CFIndex length = CFStringGetMaximumSizeForEncoding(
            CFStringGetLength(identifier),
            kCFStringEncodingUTF8
        ) + 1;
        result = malloc((size_t)length);
        if (result != NULL && !CFStringGetCString(
            identifier,
            result,
            length,
            kCFStringEncodingUTF8
        )) {
            free(result);
            result = NULL;
        }
    }
    CFRelease(information);
    return result;
}

static bool identifier_allows_hash(const char *identifier, CFDataRef hash) {
    const char *marker = ".runner.";
    const char *allowed = strstr(identifier, marker);
    if (allowed == NULL) return false;
    allowed += strlen(marker);
    CFIndex count = CFDataGetLength(hash);
    const UInt8 *bytes = CFDataGetBytePtr(hash);
    char *hex = malloc((size_t)count * 2 + 1);
    if (hex == NULL) return false;
    for (CFIndex index = 0; index < count; index++) {
        snprintf(hex + index * 2, 3, "%02x", bytes[index]);
    }
    bool found = false;
    const char *candidate = allowed;
    size_t hex_length = strlen(hex);
    while (*candidate != '\0') {
        const char *end = strchr(candidate, '.');
        size_t length = end == NULL ? strlen(candidate) : (size_t)(end - candidate);
        if (length == hex_length && strncmp(candidate, hex, length) == 0) {
            found = true;
            break;
        }
        if (end == NULL) break;
        candidate = end + 1;
    }
    free(hex);
    return found;
}

static bool suspended_child_matches(pid_t pid, const char *identifier) {
    CFNumberRef process_id = CFNumberCreate(NULL, kCFNumberIntType, &pid);
    if (process_id == NULL) return false;
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
    if (attributes == NULL) return false;
    SecCodeRef code = NULL;
    OSStatus status = SecCodeCopyGuestWithAttributes(
        NULL,
        attributes,
        kSecCSDefaultFlags,
        &code
    );
    CFRelease(attributes);
    if (status != errSecSuccess) return false;
    status = SecCodeCheckValidity(code, kSecCSStrictValidate, NULL);
    CFDictionaryRef information = status == errSecSuccess
        ? copy_signing_information(code)
        : NULL;
    CFRelease(code);
    if (information == NULL) return false;
    CFDataRef hash = CFDictionaryGetValue(information, kSecCodeInfoUnique);
    bool matches = hash != NULL && identifier_allows_hash(identifier, hash);
    CFRelease(information);
    return matches;
}

int main(int argc, char **argv) {
    char *self_path = copy_self_path();
    char *identifier = copy_self_identifier();
    if (self_path == NULL || identifier == NULL) {
        fprintf(stderr, "Launcher Bundle: could not verify launcher identity\n");
        free(self_path);
        free(identifier);
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
    if (!suspended_child_matches(pid, identifier)) {
        kill(pid, SIGKILL);
        waitpid(pid, NULL, 0);
        fprintf(stderr, "Launcher Bundle: payload identity changed\n");
        return 126;
    }

    int forwarded[] = { SIGHUP, SIGINT, SIGQUIT, SIGTERM };
    struct sigaction action = { .sa_handler = forward_signal };
    sigemptyset(&action.sa_mask);
    for (size_t index = 0; index < sizeof(forwarded) / sizeof(forwarded[0]); index++) {
        sigaction(forwarded[index], &action, NULL);
    }
    if (kill(pid, SIGCONT) != 0) {
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
    free(identifier);
    free(self_path);
    if (WIFEXITED(status)) return WEXITSTATUS(status);
    if (WIFSIGNALED(status)) {
        signal(WTERMSIG(status), SIG_DFL);
        raise(WTERMSIG(status));
        return 128 + WTERMSIG(status);
    }
    return 126;
}
