#include <xpc/xpc.h>

void av_xpc_connection_set_empty_event_handler(xpc_connection_t connection) {
    xpc_connection_set_event_handler(connection, ^(xpc_object_t event) {});
}
