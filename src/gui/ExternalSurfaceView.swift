import AppKit
import QuartzCore
import WebKit

final class ExternalSurfaceView: NSView, WKNavigationDelegate {
    private enum EmbeddedSurfaceState {
        case active
        case suspended
    }

    private enum WebContentKind {
        case blank
        case placeholder
        case external
    }

    private final class PassiveWebView: WKWebView {
        override var acceptsFirstResponder: Bool {
            false
        }

        override func becomeFirstResponder() -> Bool {
            false
        }
    }

    private final class ScanlineLayer: CALayer {
        override func draw(in context: CGContext) {
            context.setFillColor(NSColor(calibratedWhite: 1, alpha: 0.02).cgColor)
            var y: CGFloat = 0
            while y < bounds.height {
                context.fill(CGRect(x: 0, y: y, width: bounds.width, height: 1))
                y += 4
            }
        }
    }

    private struct Metrics {
        static let topInset: CGFloat = 6
        static let sideInset: CGFloat = 4
        static let spineX: CGFloat = 1
        static let textInset: CGFloat = 6
        static let labelHeight: CGFloat = 10
        static let labelGap: CGFloat = 10
        static let externalButtonSize: CGFloat = 18
        static let externalButtonTopOffset: CGFloat = -4
    }

    private struct Timing {
        static let reveal: CFTimeInterval = 0.18
        static let delay: CFTimeInterval = 0.06
    }

    private let spineLayer = CALayer()
    private let labelLayer = CATextLayer()
    private let openExternalButton = NSButton(frame: .zero)
    private let webView: PassiveWebView
    private let gradientLayer = CAGradientLayer()
    private let scanlineLayer = ScanlineLayer()
    private let washLayer = CALayer()
    private var pendingURL: URL?
    private var externalURL: URL?
    private var pendingLoadWorkItem: DispatchWorkItem?
    private var surfaceState: EmbeddedSurfaceState = .active
    private var webContentKind: WebContentKind = .blank
    private var isEyebrowLoading = false
    private var labelAnimator: LayerGlitchTextAnimator?
    private var trackingArea: NSTrackingArea?
    private var isHovering = false {
        didSet { updateOpenExternalButtonVisibility() }
    }

    override init(frame frameRect: NSRect) {
        let config = ExternalSurfaceView.makeWebConfiguration()
        webView = PassiveWebView(frame: .zero, configuration: config)
        super.init(frame: frameRect)

        wantsLayer = true
        layer = CALayer()
        layer?.backgroundColor = UIStyle.background.cgColor

        spineLayer.backgroundColor = UIStyle.spine.cgColor
        layer?.addSublayer(spineLayer)

        labelLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        labelLayer.alignmentMode = .left
        layer?.addSublayer(labelLayer)
        labelAnimator = LayerGlitchTextAnimator(
            layer: labelLayer,
            size: 10,
            baseColor: UIStyle.text.withAlphaComponent(0.20),
            glitchColor: UIStyle.accent.withAlphaComponent(0.66),
            weight: .regular,
            tracking: 1.8
        )

        if let safariImage = NSImage(
            systemSymbolName: "safari",
            accessibilityDescription: "Open in Browser"
        )?.withSymbolConfiguration(
            NSImage.SymbolConfiguration(
                pointSize: 12,
                weight: .regular,
                scale: .small
            )
        ) {
            openExternalButton.image = safariImage
        }
        openExternalButton.target = self
        openExternalButton.action = #selector(openExternalPage)
        openExternalButton.isBordered = false
        openExternalButton.imagePosition = .imageOnly
        openExternalButton.imageScaling = .scaleProportionallyDown
        openExternalButton.contentTintColor = UIStyle.text.withAlphaComponent(0.44)
        openExternalButton.toolTip = "Open in browser"
        openExternalButton.alphaValue = 0
        openExternalButton.isHidden = true
        addSubview(openExternalButton)

        webView.customUserAgent = """
        Mozilla/5.0 (iPhone; CPU iPhone OS 18_0 like Mac OS X) \
        AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 \
        Mobile/15E148 Safari/604.1
        """
        webView.wantsLayer = true
        webView.layer?.backgroundColor = UIStyle.surface.cgColor
        webView.layer?.masksToBounds = true
        webView.setValue(false, forKey: "drawsBackground")
        webView.setValue(false, forKey: "drawsTransparentBackground")
        webView.allowsMagnification = false
        webView.enclosingScrollView?.allowsMagnification = false
        webView.navigationDelegate = self
        addSubview(webView)

        gradientLayer.colors = [
            UIStyle.webOverlay.cgColor,
            NSColor(calibratedWhite: 0, alpha: 0).cgColor
        ]
        gradientLayer.startPoint = CGPoint(x: 0.5, y: 1.0)
        gradientLayer.endPoint = CGPoint(x: 0.5, y: 0.0)
        webView.layer?.addSublayer(gradientLayer)

        washLayer.backgroundColor = UIStyle.background.withAlphaComponent(0.18).cgColor
        washLayer.opacity = 0
        webView.layer?.addSublayer(washLayer)

        scanlineLayer.contentsScale = NSScreen.main?.backingScaleFactor ?? 2
        webView.layer?.addSublayer(scanlineLayer)

        updateLabel(text: "EXTERNAL")
        render(detail: nil, animated: false)
    }

    required init?(coder: NSCoder) {
        fatalError("init(coder:) has not been implemented")
    }

    override func updateTrackingAreas() {
        super.updateTrackingAreas()
        if let trackingArea {
            removeTrackingArea(trackingArea)
        }
        let trackingArea = NSTrackingArea(
            rect: bounds,
            options: [
                .activeInKeyWindow,
                .inVisibleRect,
                .mouseEnteredAndExited
            ],
            owner: self,
            userInfo: nil
        )
        addTrackingArea(trackingArea)
        self.trackingArea = trackingArea
    }

    override func mouseEntered(with event: NSEvent) {
        isHovering = true
        super.mouseEntered(with: event)
    }

    override func mouseExited(with event: NSEvent) {
        isHovering = false
        super.mouseExited(with: event)
    }

    override func layout() {
        super.layout()

        CATransaction.begin()
        CATransaction.setDisableActions(true)

        spineLayer.frame = CGRect(
            x: Metrics.spineX,
            y: 0,
            width: 1,
            height: bounds.height
        )

        let contentMinX = Metrics.spineX + Metrics.textInset

        labelLayer.frame = CGRect(
            x: contentMinX,
            y: bounds.height - Metrics.topInset - Metrics.labelHeight,
            width: max(
                bounds.width
                - contentMinX
                - Metrics.sideInset
                - Metrics.externalButtonSize,
                0
            ),
            height: Metrics.labelHeight
        )
        openExternalButton.frame = CGRect(
            x: bounds.width
                - Metrics.sideInset
                - Metrics.externalButtonSize,
            y: labelLayer.frame.minY + Metrics.externalButtonTopOffset,
            width: Metrics.externalButtonSize,
            height: Metrics.externalButtonSize
        )

        let surfaceMinX = Metrics.spineX + 1
        let containerY: CGFloat = 0
        let containerTop = labelLayer.frame.minY - Metrics.labelGap
        let surfaceFrame = CGRect(
            x: surfaceMinX,
            y: containerY,
            width: bounds.width - surfaceMinX,
            height: max(containerTop - containerY, 120)
        )
        webView.frame = surfaceFrame
        gradientLayer.frame = CGRect(
            x: 0,
            y: webView.bounds.height - 72,
            width: webView.bounds.width,
            height: 72
        )
        washLayer.frame = webView.bounds
        scanlineLayer.frame = webView.bounds
        scanlineLayer.setNeedsDisplay()

        CATransaction.commit()
    }

    func render(detail: PackageDetail?, animated: Bool = false, loading: Bool = false) {
        pendingLoadWorkItem?.cancel()
        pendingLoadWorkItem = nil
        pendingURL = nil
        externalURL = detail?.homepageURL
        updateOpenExternalButtonVisibility()
        guard let detail else {
            setInternalEyebrowLoading(false)
            updateLabel(text: "EXTERNAL")
            webView.stopLoading()
            loadBlankHTML()
            if animated {
                animateLoadTransition(after: Timing.delay)
            }
            return
        }

        updateLabel(text: "EXTERNAL")
        if let url = detail.homepageURL {
            pendingURL = url
            if surfaceState == .suspended {
                setInternalEyebrowLoading(false)
                webView.stopLoading()
                loadPlaceholderHTML(
                    title: "external surface unavailable",
                    subtitle: detail.packageName
                )
            } else {
                setInternalEyebrowLoading(true)
                loadBlankHTML()
                let workItem = DispatchWorkItem { [weak self] in
                    guard let self else { return }
                    guard self.pendingURL == url else { return }
                    self.pendingLoadWorkItem = nil
                    self.webView.stopLoading()
                    self.prepareForExternalLoad()
                    self.webView.load(URLRequest(url: url))
                }
                pendingLoadWorkItem = workItem
                DispatchQueue.main.asyncAfter(deadline: .now() + 0.35, execute: workItem)
            }
        } else {
            setInternalEyebrowLoading(loading)
            webView.stopLoading()
            if detail.hasConfiguredHomepage || loading {
                loadBlankHTML()
            } else {
                loadPlaceholderHTML(
                    title: "external surface unavailable",
                    subtitle: detail.packageName
                )
            }
        }
        if animated {
            animateLoadTransition(after: Timing.delay)
        }
    }

    func setEyebrowLoading(_ active: Bool) {
        setInternalEyebrowLoading(active)
    }

    private func updateLabel(text: String?) {
        labelAnimator?.setText(text, animated: isEyebrowLoading)
    }

    private func setInternalEyebrowLoading(_ active: Bool) {
        guard isEyebrowLoading != active else { return }
        isEyebrowLoading = active
        updateLabel(text: "EXTERNAL")
    }

    @objc private func openExternalPage() {
        guard let externalURL else { return }
        NSWorkspace.shared.open(externalURL)
    }

    private func updateOpenExternalButtonVisibility() {
        let shouldShow = isHovering && externalURL != nil
        openExternalButton.isEnabled = externalURL != nil

        if shouldShow {
            openExternalButton.isHidden = false
        }

        NSAnimationContext.runAnimationGroup { context in
            context.duration = 0.12
            openExternalButton.animator().alphaValue = shouldShow ? 1 : 0
        } completionHandler: { [weak self] in
            guard let self else { return }
            if !shouldShow {
                self.openExternalButton.isHidden = true
            }
        }
    }

    private func loadBlankHTML() {
        webContentKind = .blank
        setLoadedExternalBackground(false)
        let html = """
        <!doctype html>
        <html>
        <head>
          <meta charset="utf-8">
          <meta name="viewport"
            content="width=390, initial-scale=1, maximum-scale=1,
            user-scalable=no, viewport-fit=cover">
          <style>
            :root { color-scheme: dark; }
            html, body {
              margin: 0;
              min-height: 100vh;
              background: #121614;
            }
          </style>
        </head>
        <body></body>
        </html>
        """
        webView.loadHTMLString(html, baseURL: nil)
    }

    private func animateLoadTransition(after delay: CFTimeInterval) {
        CATransaction.begin()
        CATransaction.setDisableActions(true)
        webView.layer?.opacity = 1
        washLayer.opacity = 0
        scanlineLayer.opacity = 1
        CATransaction.commit()

        if let surfaceLayer = webView.layer {
            let containerOpacity = CABasicAnimation(keyPath: "opacity")
            containerOpacity.fromValue = 0.82
            containerOpacity.toValue = 1.0
            containerOpacity.beginTime = CACurrentMediaTime() + delay
            containerOpacity.duration = Timing.reveal
            containerOpacity.fillMode = .backwards
            containerOpacity.timingFunction = CAMediaTimingFunction(name: .easeOut)
            surfaceLayer.add(containerOpacity, forKey: "externalOpacity")
        }

        let washOpacity = CABasicAnimation(keyPath: "opacity")
        washOpacity.fromValue = 0.18
        washOpacity.toValue = 0.0
        washOpacity.beginTime = CACurrentMediaTime() + delay
        washOpacity.duration = Timing.reveal
        washOpacity.fillMode = .backwards
        washOpacity.timingFunction = CAMediaTimingFunction(name: .easeOut)
        washLayer.add(washOpacity, forKey: "externalWash")

        let scanlineOpacity = CABasicAnimation(keyPath: "opacity")
        scanlineOpacity.fromValue = 0.32
        scanlineOpacity.toValue = 1.0
        scanlineOpacity.beginTime = CACurrentMediaTime() + delay
        scanlineOpacity.duration = Timing.reveal
        scanlineOpacity.fillMode = .backwards
        scanlineOpacity.timingFunction = CAMediaTimingFunction(name: .easeOut)
        scanlineLayer.add(scanlineOpacity, forKey: "externalScanline")
    }

    private func loadPlaceholderHTML(title: String, subtitle: String) {
        webContentKind = .placeholder
        setLoadedExternalBackground(false)
        let html = """
        <!doctype html>
        <html>
        <head>
          <meta charset="utf-8">
          <meta name="viewport"
            content="width=390, initial-scale=1, maximum-scale=1,
            user-scalable=no, viewport-fit=cover">
          <style>
            :root { color-scheme: dark; }
            html, body {
              margin: 0;
              background: #121614;
              color: #d9dfda;
              font-family: "SF Mono", "Menlo", monospace;
            }
            body {
              min-height: 100vh;
              display: flex;
              align-items: center;
              justify-content: center;
            }
            .wrap {
              text-align: center;
              letter-spacing: 0.08em;
              text-transform: lowercase;
            }
            .subtitle {
              margin-top: 10px;
              opacity: 0.56;
              font-size: 12px;
            }
          </style>
        </head>
        <body>
          <div class="wrap">
            <div>\(title)</div>
            <div class="subtitle">\(subtitle)</div>
          </div>
        </body>
        </html>
        """
        webView.loadHTMLString(html, baseURL: nil)
    }

    func webView(
        _ webView: WKWebView,
        decidePolicyFor navigationAction: WKNavigationAction,
        decisionHandler: @escaping (WKNavigationActionPolicy) -> Void
    ) {
        guard shouldOpenExternally(for: navigationAction),
              let url = navigationAction.request.url
        else {
            decisionHandler(.allow)
            return
        }

        NSWorkspace.shared.open(url)
        decisionHandler(.cancel)
    }

    func webView(
        _ webView: WKWebView,
        didFail navigation: WKNavigation!,
        withError error: Error
    ) {
        handleNavigationFailure(on: webView, error: error)
    }

    func webView(
        _ webView: WKWebView,
        didFailProvisionalNavigation navigation: WKNavigation!,
        withError error: Error
    ) {
        handleNavigationFailure(on: webView, error: error)
    }

    func webView(
        _ webView: WKWebView,
        didFinish navigation: WKNavigation!
    ) {
        guard pendingLoadWorkItem == nil else { return }
        if webContentKind == .external {
            setLoadedExternalBackground(true)
        }
        setInternalEyebrowLoading(false)
    }

    func webViewWebContentProcessDidTerminate(_ webView: WKWebView) {
        suspendEmbeddedSurface()
        guard let pendingURL else {
            return
        }
        loadPlaceholderHTML(
            title: "external surface unavailable",
            subtitle: pendingURL.host ?? pendingURL.absoluteString
        )
    }

    private func handleNavigationFailure(on webView: WKWebView, error: Error) {
        let nsError = error as NSError
        guard nsError.code != NSURLErrorCancelled else {
            return
        }

        if nsError.domain == WKError.errorDomain {
            suspendEmbeddedSurface()
        }

        let subtitle: String
        if let pendingURL {
            subtitle = pendingURL.host ?? pendingURL.absoluteString
        } else {
            subtitle = "navigation failed"
        }
        loadPlaceholderHTML(
            title: surfaceState == .suspended
                ? "external surface unavailable"
                : "external surface loading…",
            subtitle: subtitle
        )
        setInternalEyebrowLoading(false)
    }

    private func suspendEmbeddedSurface() {
        pendingLoadWorkItem?.cancel()
        pendingLoadWorkItem = nil
        pendingURL = nil
        surfaceState = .suspended
        setInternalEyebrowLoading(false)
        webView.stopLoading()
    }

    private func prepareForExternalLoad() {
        webContentKind = .external
        setLoadedExternalBackground(false)
    }

    private func setLoadedExternalBackground(_ loaded: Bool) {
        webView.layer?.backgroundColor = (
            loaded ? NSColor.white : UIStyle.surface
        ).cgColor
    }

    private func shouldOpenExternally(for navigationAction: WKNavigationAction)
        -> Bool
    {
        guard navigationAction.navigationType == .linkActivated else {
            return false
        }

        guard let url = navigationAction.request.url else {
            return false
        }

        return url.scheme != "about"
    }

    private static func makeWebConfiguration() -> WKWebViewConfiguration {
        let configuration = WKWebViewConfiguration()
        let controller = WKUserContentController()
        let script = """
        (function() {
          var existing = document.querySelector('meta[name="viewport"]');
          if (!existing) {
            existing = document.createElement('meta');
            existing.name = 'viewport';
            document.head.appendChild(existing);
          }
          existing.content =
            'width=390, initial-scale=1, maximum-scale=1, user-scalable=no';
          document.documentElement.style.webkitTextSizeAdjust = '100%';
          document.body.style.overscrollBehavior = 'none';
        })();
        """
        controller.addUserScript(
            WKUserScript(
                source: script,
                injectionTime: .atDocumentEnd,
                forMainFrameOnly: true
            )
        )
        configuration.userContentController = controller

        if #available(macOS 13.0, *) {
            let preferences = WKWebpagePreferences()
            preferences.preferredContentMode = .mobile
            configuration.defaultWebpagePreferences = preferences
        }

        return configuration
    }
}
