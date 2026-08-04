#!/usr/bin/env python3
"""Generate the exact AWS commands Automic Vault may approve as read-only.

Run this with the Python bundled with AWS CLI v2 so `awscli` is importable.
The output is reviewed and committed; generation never affects runtime policy.
"""

from __future__ import annotations

import argparse
import concurrent.futures
import json
import re
import sys
import urllib.request
from collections import defaultdict
from pathlib import Path

import awscli
from awscli.clidriver import ServiceOperation, create_clidriver


SERVICE_REFERENCE_INDEX = "https://servicereference.us-east-1.amazonaws.com/"
PASSIVE_OPERATION_PREFIXES = (
    "AdminGet",
    "AdminList",
    "BatchDescribe",
    "BatchGet",
    "BatchRead",
    "Check",
    "Count",
    "Discover",
    "DomainMetadata",
    "Estimate",
    "Evaluate",
    "Get",
    "Is",
    "Lookup",
    "Preview",
    "Query",
    "Scan",
    "Search",
    "Select",
    "Validate",
    "Verify",
    "ViewBilling",
)
SENSITIVE_OPERATION = re.compile(
    r"(ApiKey|AuthorizationCode|Credential|Decrypted|ExportCertificate|Password|Presign|PrivateKey|Secret|Token|UnlockCode|VerificationCode)"
)
SENSITIVE_MEMBER_NAMES = {
    "accesstoken",
    "authorizationcode",
    "authorizationtoken",
    "authtoken",
    "basicauthcredentials",
    "bearertoken",
    "clientsecret",
    "identitytoken",
    "password",
    "privatekey",
    "randompassword",
    "refreshtoken",
    "secret",
    "secretaccesskey",
    "secretbinary",
    "secretkey",
    "secretstring",
    "sessiontoken",
    "unlockcode",
}
SENSITIVE_MEMBER_SUFFIXES = (
    "accesstoken",
    "authorizationcode",
    "authorizationtoken",
    "authtoken",
    "bearertoken",
    "clientsecret",
    "identitytoken",
    "password",
    "privatekey",
    "refreshtoken",
    "secretaccesskey",
    "secretbinary",
    "secretstring",
    "secretvalue",
    "sessiontoken",
    "unlockcode",
)
SENSITIVE_URL_NAME = re.compile(
    r"(artifact|asset|attachment|code|download|export|file|presigned|report|signed|template|upload).*url$",
    re.IGNORECASE,
)
MANUAL_EXCLUSIONS = {
    # These can expose SecureString values depending on command arguments.
    ("ssm", "get-parameter"),
    ("ssm", "get-parameter-history"),
    ("ssm", "get-parameters"),
    ("ssm", "get-parameters-by-path"),
    # This returns a time-limited download URL in Code.Location.
    ("lambda", "get-function"),
}


def load_json(url: str) -> object:
    with urllib.request.urlopen(url, timeout=60) as response:
        return json.load(response)


def reference_name(operation: ServiceOperation, references: set[str]) -> str | None:
    model = operation._operation_model.service_model
    candidates = (
        model.service_name,
        model.signing_name,
        model.endpoint_prefix,
        model.service_id.lower().replace(" ", "-"),
    )
    return next((candidate for candidate in candidates if candidate in references), None)


def output_is_sensitive(operation: ServiceOperation) -> bool:
    output = operation._operation_model.output_shape
    if output is None:
        return False
    payload = output.serialization.get("payload")
    if payload:
        payload_shape = output.members.get(payload)
        if payload_shape is not None and (
            payload_shape.type_name == "blob" or payload_shape.serialization.get("streaming")
        ):
            return True

    visited: set[str] = set()

    def visit(shape: object) -> bool:
        name = getattr(shape, "name", None)
        if name in visited:
            return False
        if name:
            visited.add(name)
        if getattr(shape, "serialization", {}).get("streaming"):
            return True
        for member_name, member in getattr(shape, "members", {}).items():
            normalized = member_name.lower()
            if (
                normalized in SENSITIVE_MEMBER_NAMES
                or normalized.endswith(SENSITIVE_MEMBER_SUFFIXES)
                or normalized.startswith("secrettoauthenticate")
                or SENSITIVE_URL_NAME.search(member_name)
            ):
                return True
            if visit(member):
                return True
        member = getattr(shape, "member", None)
        if member is not None and visit(member):
            return True
        key = getattr(shape, "key", None)
        value = getattr(shape, "value", None)
        return (key is not None and visit(key)) or (value is not None and visit(value))

    return visit(output)


def operation_is_aws_nonwrite(action: dict[str, object] | None) -> bool:
    if action is None:
        return False
    properties = action.get("Annotations", {}).get("Properties")
    return bool(
        properties
        and properties.get("IsWrite") is False
        and properties.get("IsPermissionManagement") is False
        and properties.get("IsTaggingOnly") is False
    )


def generate_catalog() -> tuple[dict[str, list[str]], int]:
    reference_index = load_json(SERVICE_REFERENCE_INDEX)
    reference_urls = {entry["service"]: entry["url"] for entry in reference_index}
    references = set(reference_urls)
    modeled_commands: list[tuple[str, str, ServiceOperation, str, str | None]] = []
    catalog: dict[str, set[str]] = defaultdict(set)
    excluded_sensitive = 0

    command_table = create_clidriver()._get_command_table()
    for service_name, service in command_table.items():
        for command_name, command in service.subcommand_table.items():
            if not isinstance(command, ServiceOperation):
                continue
            model_name = command._operation_model.name
            modeled_commands.append(
                (
                    service_name,
                    command_name,
                    command,
                    model_name,
                    reference_name(command, references),
                )
            )

    needed_references = {entry[4] for entry in modeled_commands if entry[4] is not None}

    def load_actions(reference: str) -> tuple[str, dict[str, dict[str, object]]]:
        document = load_json(reference_urls[reference])
        return reference, {action["Name"]: action for action in document.get("Actions", [])}

    with concurrent.futures.ThreadPoolExecutor(max_workers=24) as executor:
        reference_actions = dict(executor.map(load_actions, needed_references))

    for service_name, command_name, command, model_name, reference in modeled_commands:
        if (service_name, command_name) in MANUAL_EXCLUSIONS or output_is_sensitive(command):
            excluded_sensitive += 1
            continue

        # Preserve existing exact List/Describe/Head behavior without allowing
        # unrecognized future commands merely because their name looks safe.
        if model_name.startswith(("List", "Describe")) or (
            service_name == "s3api" and model_name.startswith("Head")
        ):
            catalog[service_name].add(command_name)
            continue

        if service_name in {"iam", "sts"}:
            continue
        if reference is None:
            continue
        action = reference_actions[reference].get(model_name)
        if not operation_is_aws_nonwrite(action):
            continue
        if not model_name.startswith(PASSIVE_OPERATION_PREFIXES):
            continue
        if SENSITIVE_OPERATION.search(model_name):
            excluded_sensitive += 1
            continue
        catalog[service_name].add(command_name)

    # The hardened wrapper handles this custom, read-only command itself.
    catalog["s3"].add("ls")
    # This is the only STS operation that neither creates nor returns credentials.
    catalog["sts"].add("get-caller-identity")
    return {service: sorted(commands) for service, commands in sorted(catalog.items())}, excluded_sensitive


def render(catalog: dict[str, list[str]]) -> str:
    lines = [
        "// Generated by scripts/generate-aws-read-only-catalog.py; do not edit.",
        f"// AWS CLI {awscli.__version__}; unknown commands deliberately fail closed.",
        "",
        "public func awsCommandIsReadOnly(service: String, operation: String) -> Bool {",
        "    awsReadOnlyOperations[service.lowercased()]?.contains(operation.lowercased()) == true",
        "}",
        "",
        "private let awsReadOnlyOperations: [String: Set<String>] = [",
    ]
    for service, commands in catalog.items():
        lines.append(f'    "{service}": [')
        lines.extend(f'        "{command}",' for command in commands)
        lines.append("    ],")
    lines.extend(["]", ""])
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--output",
        type=Path,
        default=Path("src/menu-helper/Sources/MenubarHelperCore/AWSReadOnlyCommandPolicy.swift"),
    )
    args = parser.parse_args()
    catalog, excluded_sensitive = generate_catalog()
    args.output.write_text(render(catalog))
    print(
        f"wrote {sum(map(len, catalog.values()))} commands across {len(catalog)} services; "
        f"kept {excluded_sensitive} sensitive candidates gated",
        file=sys.stderr,
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
