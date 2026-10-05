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

// Synthetic presentation data only; never load the user's Authorization History.
private struct AccessRequestRecord: Equatable {
    var commandForDisplay = "aws s3 ls"
    var decision = "Approved"
    var launcher: String? = "Fixture"
    var approvalSourceLabel = "Policy"
    var date = Date(timeIntervalSince1970: 1_790_000_000)
}
private func localizedUIString(_ text: String) -> String { text }
private final class FlippedRoot: NSView { override var isFlipped: Bool { true } }
'''
fixture += declaration("private extension View {\n    func outlinedPill") + "\n"
fixture += declaration("private struct ToolActivityPopover") + "\n"
fixture += declaration("private struct ActivityHoverSelection") + "\n"
fixture += declaration("private struct InstantActivityPopover") + "\n"
fixture += r'''
@MainActor private final class PreviewModel: ObservableObject {
    @Published var x: CGFloat = 0.5
    @Published var show = false
    @Published var command = "aws s3 ls"
}
private struct Preview: View {
    @ObservedObject var model: PreviewModel
    var body: some View {
        VStack {
            Spacer().frame(height: 200)
            Rectangle().frame(width: 400, height: 8)
                .background {
                    InstantActivityPopover(content: model.show ? ToolActivityPopover(record: AccessRequestRecord(commandForDisplay: model.command), interval: "Fixture interval") : nil, anchorX: model.x)
                        .frame(width: 400, height: 8)
                }
            Spacer()
        }.frame(width: 800, height: 600)
    }
}
'''
fixture += r'''
MainActor.assumeIsolated {
    setbuf(stdout, nil)
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
    let root = FlippedRoot(frame: NSRect(x: 0, y: 0, width: 800, height: 300))
    window.contentView = root
    let anchor = InstantActivityPopover.AnchorView(frame: NSRect(x: 40, y: 80, width: 400, height: 8))
    root.addSubview(anchor)
    window.orderFront(nil)
    anchor.anchorX = 0.5
    anchor.content = ToolActivityPopover(record: AccessRequestRecord(), interval: "Fixture interval")
    anchor.refresh()
    precondition(!anchor.popover.animates && anchor.popover.isShown)
    precondition(anchor.popover.positioningRect == NSRect(x: 200, y: 0, width: 1, height: 8))
    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    let initialFrame = anchor.popover.contentViewController!.view.window!.frame
    anchor.anchorX = 0.6
    anchor.refresh()
    RunLoop.current.run(until: Date().addingTimeInterval(0.05))
    let pannedFrame = anchor.popover.contentViewController!.view.window!.frame
    precondition(abs(pannedFrame.minY - initialFrame.minY) < 1,
                 "Horizontal panning must not move the popover vertically or flip its edge")
    precondition(abs((pannedFrame.midX - initialFrame.midX) - 40.1) < 1,
                 "Popover must track the selected bar horizontally")
    anchor.anchorX = 0.5
    anchor.frame = NSRect(x: 70, y: 160, width: 600, height: 8)
    anchor.needsLayout = true
    anchor.layoutSubtreeIfNeeded()
    precondition(anchor.popover.positioningRect == NSRect(x: 300, y: 0, width: 1, height: 8),
                 "Anchor must follow the bar width after layout, in local coordinates")
    anchor.content = nil
    anchor.refresh()
    precondition(!anchor.popover.isShown, "Dismissal must be immediate")
    anchor.content = ToolActivityPopover(record: AccessRequestRecord(), interval: "Fixture interval")
    anchor.refresh()
    anchor.removeFromSuperview()
    precondition(!anchor.popover.isShown, "Detached timelines must dismiss their popover")
    window.orderOut(nil)

    let model = PreviewModel()
    let host = NSHostingView(rootView: Preview(model: model))
    let swiftWindow = NSWindow(contentRect: NSRect(x: 300, y: 400, width: 800, height: 600),
                              styleMask: [.titled, .fullSizeContentView], backing: .buffered, defer: false)
    swiftWindow.contentView = host
    swiftWindow.orderFront(nil)
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    func findAnchor(_ view: NSView) -> InstantActivityPopover.AnchorView? {
        if let anchor = view as? InstantActivityPopover.AnchorView { return anchor }
        return view.subviews.lazy.compactMap { findAnchor($0) }.first
    }
    let liveAnchor = findAnchor(host)!
    model.show = true
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    let beforePan = liveAnchor.popover.contentViewController!.view.window!.frame
    let barOnScreen = swiftWindow.convertToScreen(liveAnchor.convert(liveAnchor.bounds, to: nil))
    precondition(abs(beforePan.maxY - barOnScreen.minY) < 1,
                 "Popover arrow must meet the timeline on initial display")
    precondition(abs(beforePan.midX - barOnScreen.midX) < 1)

    model.x = 0.6
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    let afterPan = liveAnchor.popover.contentViewController!.view.window!.frame
    precondition(abs(beforePan.minY - afterPan.minY) < 1, "SwiftUI panning must preserve vertical position")
    precondition(abs(afterPan.midX - beforePan.midX - 40.1) < 1, "SwiftUI panning must follow the bar")
    for x: CGFloat in [0.4, 0.65, 0.45, 0.55] {
        model.x = x
        RunLoop.current.run(until: Date().addingTimeInterval(0.03))
        let frame = liveAnchor.popover.contentViewController!.view.window!.frame
        precondition(abs(frame.maxY - barOnScreen.minY) < 1)
        precondition(abs(frame.midX - (barOnScreen.minX + 401 * x)) < 1)
    }
    model.command = (1...14).map { "argument-\($0)" }.joined(separator: "\n")
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    let expandedFrame = liveAnchor.popover.contentViewController!.view.window!.frame
    precondition(expandedFrame.height > beforePan.height + 150)
    precondition(abs(expandedFrame.maxY - barOnScreen.minY) < 1,
                 "A longer command must grow away from the anchored bar")
    model.command = "aws s3 ls"
    RunLoop.current.run(until: Date().addingTimeInterval(0.1))
    let contractedFrame = liveAnchor.popover.contentViewController!.view.window!.frame
    precondition(abs(contractedFrame.height - beforePan.height) < 1)
    precondition(abs(contractedFrame.maxY - barOnScreen.minY) < 1)
    let shortContent = ToolActivityPopover(record: AccessRequestRecord(), interval: "Fixture interval")
    var longRecord = AccessRequestRecord()
    longRecord.commandForDisplay = (1...14).map { "argument-\($0)" }.joined(separator: "\n")
    let longContent = ToolActivityPopover(record: longRecord, interval: "Fixture interval")
    let shortRender = ImageRenderer(content: shortContent).nsImage!
    let longRender = ImageRenderer(content: longContent).nsImage!
    precondition(longRender.size.height > shortRender.size.height + 150,
                 "The command must expand beyond three lines without truncation")
    swiftWindow.orderOut(nil)
    print("Overview hover buffer, precise panning, anchor layout, and instant dismissal passed")
}
'''
with tempfile.TemporaryDirectory(prefix="av-overview-hover-") as directory:
    swift = Path(directory) / "main.swift"
    binary = Path(directory) / "check"
    swift.write_text(fixture)
    subprocess.run(["swiftc", str(swift), "-o", str(binary)], check=True)
    subprocess.run([str(binary)], check=True, timeout=20)
