import Foundation
import XPC

private let approvalService = "com.automicvault.av2.approval"
private let menuHelperRequirement = """
anchor apple generic and certificate leaf[subject.OU] = ZU76A67LGU and \
identifier "com.automicvault"
"""

private func fail(_ message: String) -> Never {
    FileHandle.standardError.write(Data("Automic Vault Varlock plugin: \(message)\n".utf8))
    exit(1)
}

guard CommandLine.arguments.count == 2 else {
    fail("expected one Secret Name")
}
let secretName = CommandLine.arguments[1]
let bytes = Array(secretName.utf8)
guard let first = bytes.first,
      first == 95 || 65...90 ~= first || 97...122 ~= first,
      bytes.dropFirst().allSatisfy({
          $0 == 95 || 48...57 ~= $0 || 65...90 ~= $0 || 97...122 ~= $0
      })
else {
    fail("invalid Secret Name")
}
let cwd = URL(fileURLWithPath: FileManager.default.currentDirectoryPath, isDirectory: true)
    .resolvingSymlinksInPath().path

let connection = xpc_connection_create_mach_service(approvalService, nil, 0)
guard xpc_connection_set_peer_code_signing_requirement(
    connection, menuHelperRequirement
) == 0 else {
    fail("could not verify Automic Vault")
}
xpc_connection_set_event_handler(connection) { _ in }
xpc_connection_activate(connection)
defer { xpc_connection_cancel(connection) }

let request = xpc_dictionary_create_empty()
xpc_dictionary_set_string(request, "op", "varlock")
xpc_dictionary_set_string(request, "key", secretName)
xpc_dictionary_set_string(request, "cwd", cwd)

let reply = xpc_connection_send_message_with_reply_sync(connection, request)
if xpc_get_type(reply) == XPC_TYPE_ERROR {
    let error = xpc_dictionary_get_string(reply, XPC_ERROR_KEY_DESCRIPTION)
        .map(String.init(cString:)) ?? "XPC connection failed"
    fail(error)
}
guard xpc_dictionary_get_bool(reply, "ok") else {
    let error = xpc_dictionary_get_string(reply, "error")
        .map(String.init(cString:)) ?? "request denied"
    fail(error)
}
guard let value = xpc_dictionary_get_string(reply, "value") else {
    fail("Automic Vault returned no Secret Value")
}
FileHandle.standardOutput.write(Data(String(cString: value).utf8))
