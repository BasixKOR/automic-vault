#!/usr/bin/env python3
"""Check the real overview hover selection and native popover geometry without input automation."""
from pathlib import Path
import subprocess
import tempfile

source = (Path(__file__).resolve().parents[1] /
          "src/menu-helper/Sources/MenubarHelper/MainWindow.swift").read_text()


def declaration(marker):
    start = source.index(marker)
    end = source.index("{", start) + 1
    depth = 1
    while depth:
        depth += (source[end] == "{") - (source[end] == "}")
        end += 1
    return source[start:end]


fixture = r'''
import SwiftUI
import AppKit

// Content is irrelevant to anchoring; keep the fixture free of Authorization History.
private struct ToolActivityPopover: View {
    var body: some View { Text("Fixture").frame(width: 320, height: 130) }
}
'''
fixture += declaration("private struct ActivityHoverSelection") + "\n"
fixture += declaration("private struct InstantActivityPopover") + "\n"
fixture += r'''
MainActor.assumeIsolated {
    var hover = ActivityHoverSelection()
    let counts = [0, 0, 1, 0, 1, 0, 0, 0, 0, 0]
    // Ten-point pitch; populated bars occupy 20...29 and 40...49.
    hover.update(x: 14, width: 99, counts: counts)
    precondition(hover.slot == 2, "Initial entry must catch an isolated event six points away")
    hover.update(x: 14, width: 99, counts: counts)
    precondition(hover.slot == 2, "Vertical entry/exit drift must retain the selected event")
    hover.update(x: 15, width: 99, counts: counts)
    precondition(hover.slot == nil, "Panning must not snap across empty buckets")
    hover.update(x: 22, width: 99, counts: counts)
    precondition(hover.slot == 2)
    hover.update(x: 39, width: 99, counts: counts)
    precondition(hover.slot == nil, "Panning must not acquire the adjacent event early")
    hover.update(x: 40, width: 99, counts: counts)
    precondition(hover.slot == 4)
    hover.update(x: 50, width: 99, counts: counts)
    precondition(hover.slot == nil, "Panning must release at the bucket boundary")
    hover = ActivityHoverSelection()
    hover.update(x: 13.9, width: 99, counts: counts)
    precondition(hover.slot == nil, "Acquisition buffer must be bounded")
    hover = ActivityHoverSelection()
    hover.update(x: 35, width: 99, counts: counts)
    precondition(hover.slot == 4, "Entry must choose the nearest populated bar")
    hover.update(x: 35, width: 0, counts: [])
    precondition(hover.slot == nil)

    _ = NSApplication.shared
    let window = NSWindow(contentRect: NSRect(x: 300, y: 400, width: 800, height: 300),
                          styleMask: [.titled], backing: .buffered, defer: false)
    let root = NSView(frame: NSRect(x: 0, y: 0, width: 800, height: 300))
    window.contentView = root
    let anchor = InstantActivityPopover.AnchorView(frame: NSRect(x: 40, y: 80, width: 400, height: 8))
    root.addSubview(anchor)
    window.orderFront(nil)
    anchor.anchorX = 0.5
    anchor.content = ToolActivityPopover()
    anchor.refresh()
    precondition(!anchor.popover.animates && anchor.popover.isShown)
    precondition(anchor.popover.positioningRect == NSRect(x: 240, y: 80, width: 1, height: 8))
    anchor.frame = NSRect(x: 70, y: 160, width: 600, height: 8)
    anchor.needsLayout = true
    anchor.layoutSubtreeIfNeeded()
    precondition(anchor.popover.positioningRect == NSRect(x: 370, y: 160, width: 1, height: 8),
                 "Anchor must follow the bar after layout, in window content coordinates")
    anchor.content = nil
    anchor.refresh()
    precondition(!anchor.popover.isShown, "Dismissal must be immediate")
    anchor.content = ToolActivityPopover()
    anchor.refresh()
    anchor.removeFromSuperview()
    precondition(!anchor.popover.isShown, "Detached timelines must dismiss their popover")
    window.orderOut(nil)
    print("Overview hover buffer, precise panning, anchor layout, and instant dismissal passed")
}
'''
with tempfile.TemporaryDirectory(prefix="av-overview-hover-") as directory:
    swift = Path(directory) / "main.swift"
    binary = Path(directory) / "check"
    swift.write_text(fixture)
    subprocess.run(["swiftc", str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=20)
