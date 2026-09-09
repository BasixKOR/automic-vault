#!/usr/bin/env python3
"""Manually reproduce uv native-auth disclosure on macOS with a dummy credential.

Usage: python3 scripts/check-uv-keychain.py /absolute/path/to/reviewed/uv
May briefly show a Keychain prompt for security; do not approve it. This is an
upstream behavior check, never part of a Detector or an unattended test suite.
"""

import os
from pathlib import Path
import subprocess
import sys
import tempfile
import uuid


def main():
    if sys.platform != "darwin" or len(sys.argv) != 2:
        sys.exit(__doc__)
    uv = str(Path(sys.argv[1]).resolve(strict=True))
    subprocess.run(["/usr/bin/codesign", "--verify", "--strict", uv], check=True)
    subprocess.run([uv, "--version"], check=True)
    host = f"av-uv-check-{uuid.uuid4().hex}.invalid"
    dummy = f"synthetic-{uuid.uuid4().hex}"
    env = {k: v for k, v in os.environ.items() if not k.startswith("UV_")}
    env["UV_PREVIEW_FEATURES"] = "native-auth"

    with tempfile.TemporaryDirectory(prefix="av-uv-check-") as root:
        env["UV_CREDENTIALS_DIR"] = root
        base = [uv, "--no-config", "--offline", "--no-cache", "auth"]

        def run(args, **kwargs):
            return subprocess.run(
                args, env=env, cwd=root, capture_output=True, text=True,
                timeout=5, **kwargs,
            )

        item = ["/usr/bin/security", "find-generic-password",
                "-s", f"uv:{host}", "-a", "__token__"]
        print(f"Disposable service: uv:{host}", flush=True)
        # Check absence before allowing the finally block to remove this item.
        absent = run(item)
        assert absent.returncode == 44, "Cannot establish dummy item is absent"
        try:
            run(base + ["login", host, "--token", "-"],
                input=dummy + "\n").check_returncode()
            run(item).check_returncode()  # Metadata only: prove Keychain storage.
            assert not (Path(root) / "credentials.toml").exists(), "Plaintext fallback"
            result = run(base + ["token", host])
            result.check_returncode()
            assert result.stdout.rstrip("\n") == dummy, "Disclosure behavior changed"
            print("CONFIRMED: uv auth token returned the dummy credential", flush=True)
            try:
                result = run(item + ["-w"])
                if result.returncode == 0:
                    assert result.stdout.rstrip("\n") == dummy, "Unexpected credential"
                    print("EXPOSED: /usr/bin/security also returned the dummy")
                else:
                    print(f"security returned no credential (exit {result.returncode})")
            except subprocess.TimeoutExpired:
                print("security timed out; no approval given. Timeout alone is inconclusive.")
        finally:
            run(base + ["logout", host]).check_returncode()
            assert run(item).returncode == 44, "Dummy item cleanup could not be verified"
            print("Dummy credential removed")


if __name__ == "__main__":
    main()
