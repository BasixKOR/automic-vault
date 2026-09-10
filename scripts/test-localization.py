#!/usr/bin/env python3
"""Check shipped localization resources and native language selection in a fresh app process."""
import argparse
import collections
import json
from pathlib import Path
import plistlib
import re
import shutil
import subprocess
import tempfile

repo = Path(__file__).resolve().parent.parent
helper = repo / "src/menu-helper"
resources = helper / "Resources"
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--signed", action="store_true")
parser.add_argument("binary", type=Path)
args = parser.parse_args()


def strings(path):
    subprocess.run(["plutil", "-lint", str(path)], check=True, capture_output=True)
    pairs = re.findall(r'^("(?:[^"\\]|\\.)*") = ("(?:[^"\\]|\\.)*");$', path.read_text(), re.M)
    result = {json.loads(key): json.loads(value) for key, value in pairs}
    assert len(result) == len(pairs), f"Duplicate keys in {path}"
    return result


english = strings(resources / "en.lproj/Localizable.strings")
chinese = strings(resources / "zh-Hans.lproj/Localizable.strings")
assert english.keys() == chinese.keys()
for key, translation in chinese.items():
    assert english[key] == key and translation.strip(), key
    # Compare placeholder multisets so a translation cannot drop, duplicate, or
    # reinterpret an identifier or a numeric status. These tables use no reordering.
    placeholders = r"%(?:\d+\$)?(?:lld|llu|ld|lu|d|u|@|%)"
    assert collections.Counter(re.findall(placeholders, key)) == collections.Counter(re.findall(placeholders, translation)), key
    assert re.findall(r"`([^`]+)`", key) == re.findall(r"`([^`]+)`", translation), key

with tempfile.TemporaryDirectory(prefix="av-localization-") as temporary:
    binary = args.binary.resolve()
    if args.signed:
        # The release signature seals its Info.plist and resources. Test that
        # bundle in place; reconstructing it invalidates the signature.
        app = binary.parent.parent
        assert binary.parent.name == "MacOS" and app.name == "Contents"
        assert app.parent.suffix == ".app"
        subprocess.run(["codesign", "--verify", "--strict", str(app.parent)], check=True)
        for language, expected in [("en", english), ("zh-Hans", chinese)]:
            assert strings(app / "Resources" / f"{language}.lproj/Localizable.strings") == expected
    else:
        app = Path(temporary) / "Automic Vault.app/Contents"
        (app / "MacOS").mkdir(parents=True)
        (app / "Resources").mkdir()
        shutil.copy2(helper / "Info.plist", app / "Info.plist")
        for localization in resources.glob("*.lproj"):
            shutil.copytree(localization, app / "Resources" / localization.name)
        shutil.copy2(binary, app / "MacOS/AutomicVaultMenubar")
        binary = app / "MacOS/AutomicVaultMenubar"
    with (app / "Info.plist").open("rb") as file:
        info = plistlib.load(file)
    assert info["CFBundleDevelopmentRegion"] == "en"
    assert info["CFBundleLocalizations"] == ["en", "zh-Hans"]
    for preferences, expect_chinese in [
        (["zh-Hans"], True),
        (["zh-CN"], True),
        (["fr", "zh-Hans", "en"], True),
        (["en", "zh-Hans"], False),
        (["fr", "en"], False),
    ]:
        subprocess.run([
            str(binary), "--self-check-localization",
            *(["--expect-chinese"] if expect_chinese else []),
            "-AppleLanguages", "(" + ",".join(preferences) + ")",
        ], check=True, timeout=30)

print(f"Localization checks passed ({len(chinese)} strings, 5 language preferences)")
