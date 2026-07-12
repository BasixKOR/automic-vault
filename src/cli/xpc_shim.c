#include <xpc/xpc.h>
#include <stdio.h>
#include <string.h>

void av_xpc_connection_set_empty_event_handler(xpc_connection_t connection) {
    xpc_connection_set_event_handler(connection, ^(xpc_object_t event) {
        if (xpc_get_type(event) != XPC_TYPE_DICTIONARY) return;
        const char *name = xpc_dictionary_get_string(event, "event");
        if (name != NULL && strcmp(name, "human-approval-required") == 0) {
            fputs("automic vault: human approval required\n", stderr);
            fflush(stderr);
        }
    });
}
