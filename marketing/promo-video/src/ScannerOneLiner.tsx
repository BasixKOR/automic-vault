import {
  AbsoluteFill,
  Easing,
  Img,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

export const scannerOneLinerDurationInFrames = sec(21.5);

const red = "#d83a2f";
const green = "#6bffb0";
const amber = "#ffb347";
const blue = "#6aa9ff";
const black = "#030506";
const panel = "rgba(18, 25, 29, 0.82)";
const panelStrong = "rgba(23, 33, 38, 0.94)";
const ink = "#d6c7a1";
const inkMuted = "#b89b73";
const line = "rgba(214, 199, 161, 0.34)";
const lineFaint = "rgba(214, 199, 161, 0.18)";
const display =
  '"Barlow Condensed", "Arial Narrow", Impact, ui-sans-serif, system-ui, sans-serif';
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';

const scannerCommand =
  "/usr/bin/curl -fsSL https://www.automicvault.com/scanner.sh | /bin/bash";
const installCommand =
  "/usr/bin/curl -fsSL https://www.automicvault.com/install.sh | /bin/bash";

const clamp = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const easeOut = Easing.bezier(0.16, 1, 0.3, 1);

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], {
    ...clamp,
    easing: easeOut,
  });

const fadeOut = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [1, 0], {
    ...clamp,
    easing: Easing.bezier(0.7, 0, 0.84, 0),
  });

const softY = (frame: number, start: number, end: number, from: number) =>
  interpolate(frame, [start, end], [from, 0], {
    ...clamp,
    easing: easeOut,
  });

const typeText = (text: string, frame: number, start: number, duration: number) => {
  if (frame < start) {
    return "";
  }

  const progress = Math.min(1, (frame - start + 1) / duration);
  return text.slice(0, Math.ceil(text.length * progress));
};

const Background: React.FC = () => {
  const frame = useCurrentFrame();
  const drift = interpolate(frame, [0, scannerOneLinerDurationInFrames], [0, 1], clamp);
  const x = interpolate(drift, [0, 1], [-38, 44], clamp);
  const y = interpolate(drift, [0, 1], [24, -32], clamp);

  return (
    <AbsoluteFill style={{ background: black, overflow: "hidden" }}>
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(115deg, #030506 0%, #0a0d10 50%, #030506 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          inset: -160,
          opacity: 0.52,
          filter: "blur(60px)",
          transform: `translate(${x}px, ${y}px) rotate(-6deg)`,
          background:
            "radial-gradient(circle at 25% 28%, rgba(255,179,71,0.18), transparent 20%), radial-gradient(circle at 84% 58%, rgba(216,58,47,0.22), transparent 22%), linear-gradient(120deg, transparent 0%, rgba(106,169,255,0.08) 44%, transparent 72%)",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18,
          background:
            "repeating-linear-gradient(0deg, rgba(214,199,161,0.034) 0, rgba(214,199,161,0.034) 1px, transparent 1px, transparent 7px), repeating-linear-gradient(90deg, rgba(214,199,161,0.018) 0, rgba(214,199,161,0.018) 1px, transparent 1px, transparent 72px)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(circle at center, transparent 42%, rgba(0,0,0,0.68) 84%), linear-gradient(180deg, rgba(0,0,0,0.14), rgba(0,0,0,0.52))",
        }}
      />
    </AbsoluteFill>
  );
};

const Brand: React.FC = () => (
  <div
    style={{
      position: "absolute",
      left: 70,
      top: 54,
      display: "flex",
      alignItems: "center",
      gap: 18,
    }}
  >
    <Img
      src={staticFile("icon.png")}
      style={{
        width: 68,
        height: 68,
        objectFit: "contain",
        filter: "drop-shadow(0 0 24px rgba(216,58,47,0.22))",
      }}
    />
    <Img
      src={staticFile("site-wordmark.webp")}
      style={{
        width: 238,
        height: "auto",
        objectFit: "contain",
        transform: "translateY(5px)",
        filter: "drop-shadow(0 10px 18px rgba(0,0,0,0.45))",
      }}
    />
  </div>
);

const MessageCard: React.FC<{
  start: number;
  end: number;
  kicker: string;
  title: string;
  body: string;
  accent?: string;
}> = ({ start, end, kicker, title, body, accent = amber }) => {
  const frame = useCurrentFrame();
  const opacity = fade(frame, start, start + 16) * fadeOut(frame, end - 16, end);
  const y = softY(frame, start, start + 18, 22);

  return (
    <div
      style={{
        position: "absolute",
        left: 78,
        top: 180,
        width: 598,
        opacity,
        transform: `translateY(${y}px)`,
      }}
    >
      <div
        style={{
          color: accent,
          fontFamily: mono,
          fontSize: 22,
          fontWeight: 800,
          letterSpacing: 0,
          textTransform: "uppercase",
        }}
      >
        {kicker}
      </div>
      <div
        style={{
          marginTop: 26,
          color: ink,
          fontFamily: display,
          fontSize: 94,
          fontWeight: 800,
          letterSpacing: 0,
          lineHeight: 0.9,
          textTransform: "uppercase",
          textShadow: "0 16px 34px rgba(0,0,0,0.58)",
        }}
      >
        {title}
      </div>
      <div
        style={{
          width: 560,
          marginTop: 28,
          paddingTop: 24,
          borderTop: `1px solid ${line}`,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 28,
          fontWeight: 600,
          lineHeight: 1.52,
        }}
      >
        {body}
      </div>
    </div>
  );
};

type TerminalLineKind = "prompt" | "muted" | "step" | "ok" | "error" | "heading" | "plain";

const outputLines: Array<{
  at: number;
  text: string;
  kind?: TerminalLineKind;
  indent?: number;
}> = [
  { at: 112, text: "╭─ Automic Vault scanner", kind: "heading" },
  { at: 126, text: "│ detector-only secret exposure check", kind: "muted" },
  { at: 146, text: "│ ◆ Downloading scanner", kind: "step" },
  { at: 166, text: "│ ◆ Unpacking scanner", kind: "step" },
  { at: 188, text: "│ ✓ Writes denied", kind: "ok" },
  { at: 208, text: "│ ✓ Network denied", kind: "ok" },
  { at: 228, text: "│ ◆ Running package-specific detectors", kind: "step" },
  { at: 260, text: "│", kind: "muted" },
  { at: 276, text: "│ ✗ 5 plaintext credential findings", kind: "error" },
  { at: 294, text: "│   Scope     isotope detectors only", kind: "muted" },
  { at: 312, text: "│   Checked   130 isotope detectors", kind: "muted" },
  { at: 330, text: "│   Warnings  0 warnings", kind: "muted" },
  { at: 354, text: "│", kind: "muted" },
  { at: 370, text: "│ Findings", kind: "heading" },
  { at: 390, text: "│   1. high isotope:curl", kind: "error" },
  {
    at: 408,
    text: "│      netrc file contains plaintext credentials: /Users/dev/.netrc",
    kind: "plain",
  },
  { at: 430, text: "│   2. high isotope:openssh", kind: "error" },
  {
    at: 448,
    text: "│      private key is stored without passphrase encryption",
    kind: "plain",
  },
  { at: 472, text: "│", kind: "muted" },
  { at: 488, text: "╰─ ✓ Scan complete", kind: "ok" },
  { at: 520, text: "╭─ Next step", kind: "heading" },
  { at: 538, text: "│ Fix these findings with Automic Vault.", kind: "heading" },
  { at: 560, text: `│ ${installCommand}`, kind: "step" },
  { at: 582, text: "╰─", kind: "muted" },
];

const colorForKind = (kind: TerminalLineKind | undefined) => {
  switch (kind) {
    case "prompt":
      return blue;
    case "step":
      return blue;
    case "ok":
      return green;
    case "error":
      return red;
    case "heading":
      return ink;
    case "muted":
      return inkMuted;
    default:
      return ink;
  }
};

const TerminalPanel: React.FC = () => {
  const frame = useCurrentFrame();
  const enterFlash = fade(frame, 86, 94) * fadeOut(frame, 96, 112);
  const terminalY = softY(frame, 24, 46, 34);
  const terminalScale = interpolate(frame, [0, 46], [0.975, 1], {
    ...clamp,
    easing: easeOut,
  });
  const command = typeText(scannerCommand, frame, 36, 52);
  const promptDone = frame >= 88;

  return (
    <div
      style={{
        position: "absolute",
        right: 70,
        top: 142,
        width: 1078,
        height: 788,
        opacity: fade(frame, 12, 32),
        transform: `translateY(${terminalY}px) scale(${terminalScale})`,
        transformOrigin: "center center",
        border: `1px solid ${line}`,
        borderRadius: 10,
        overflow: "hidden",
        background: panel,
        boxShadow:
          "0 34px 92px rgba(0,0,0,0.52), inset 0 1px 0 rgba(214,199,161,0.12)",
      }}
    >
      <div
        style={{
          height: 58,
          display: "flex",
          alignItems: "center",
          gap: 12,
          padding: "0 22px",
          borderBottom: `1px solid ${lineFaint}`,
          background: "rgba(3,5,6,0.72)",
        }}
      >
        {[red, amber, green].map((dot) => (
          <span
            key={dot}
            style={{
              width: 13,
              height: 13,
              borderRadius: 999,
              background: dot,
              boxShadow: `0 0 14px ${dot}55`,
            }}
          />
        ))}
        <span
          style={{
            marginLeft: 12,
            color: inkMuted,
            fontFamily: mono,
            fontSize: 19,
            fontWeight: 700,
          }}
        >
          sandboxed scanner
        </span>
      </div>

      <div
        style={{
          position: "relative",
          height: 730,
          padding: "26px 30px",
          background:
            "linear-gradient(180deg, rgba(3,5,6,0.3), transparent 32%), rgba(10,13,16,0.58)",
        }}
      >
        <div
          style={{
            display: "flex",
            alignItems: "flex-start",
            gap: 13,
            color: ink,
            fontFamily: mono,
            fontSize: 26,
            fontWeight: 800,
            lineHeight: 1.4,
          }}
        >
          <span style={{ color: blue }}>$</span>
          <span style={{ color: ink }}>{command}</span>
          {!promptDone ? (
            <span
              style={{
                width: 12,
                height: 35,
                marginTop: 2,
                borderRadius: 2,
                background: green,
                opacity: Math.floor(frame / 8) % 2 ? 0.25 : 1,
                boxShadow: "0 0 18px rgba(107,255,176,0.5)",
              }}
            />
          ) : null}
        </div>

        <div
          style={{
            position: "absolute",
            left: 30,
            right: 30,
            top: 83,
            height: 1,
            opacity: enterFlash,
            background: `linear-gradient(90deg, transparent, ${green}, transparent)`,
            boxShadow: `0 0 28px ${green}`,
          }}
        />

        <div style={{ position: "absolute", left: 30, right: 30, top: 104 }}>
          {outputLines.map((line, index) => {
            const lineFrame = frame - line.at;
            const opacity = fade(frame, line.at, line.at + 10);
            const y = interpolate(lineFrame, [0, 10], [12, 0], {
              ...clamp,
              easing: easeOut,
            });
            const color = colorForKind(line.kind);

            return (
              <div
                key={`${line.text}-${index}`}
                style={{
                  color,
                  opacity,
                  transform: `translateY(${y}px)`,
                  fontFamily: mono,
                  fontSize: line.text.includes(installCommand) ? 22 : 24,
                  fontWeight:
                    line.kind === "heading" || line.kind === "error" || line.kind === "ok"
                      ? 800
                      : 650,
                  lineHeight: "31px",
                  whiteSpace: "nowrap",
                  textShadow:
                    line.kind === "error"
                      ? "0 0 18px rgba(216,58,47,0.28)"
                      : line.kind === "ok"
                        ? "0 0 18px rgba(107,255,176,0.2)"
                        : "none",
                }}
              >
                {line.text}
              </div>
            );
          })}
        </div>
      </div>
    </div>
  );
};

const CommandStrip: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity = fade(frame, 472, 500);

  return (
    <div
      style={{
        position: "absolute",
        left: 78,
        right: 78,
        bottom: 58,
        opacity,
        display: "grid",
        gridTemplateColumns: "auto 1fr",
        gap: 20,
        alignItems: "center",
        padding: "22px 28px",
        border: `1px solid rgba(255,179,71,0.58)`,
        borderLeft: `5px solid ${amber}`,
        background:
          "linear-gradient(90deg, rgba(255,179,71,0.15), transparent 56%), rgba(18,25,29,0.88)",
        boxShadow: "0 24px 56px rgba(0,0,0,0.34)",
      }}
    >
      <div
        style={{
          color: amber,
          fontFamily: mono,
          fontSize: 20,
          fontWeight: 900,
          textTransform: "uppercase",
        }}
      >
        Scan now
      </div>
      <div
        style={{
          color: ink,
          fontFamily: mono,
          fontSize: 25,
          fontWeight: 800,
          whiteSpace: "nowrap",
          overflow: "hidden",
          textOverflow: "ellipsis",
        }}
      >
        {scannerCommand}
      </div>
    </div>
  );
};

const EndCard: React.FC = () => {
  const frame = useCurrentFrame();
  const start = 572;
  const opacity = fade(frame, start, start + 18);
  const y = softY(frame, start, start + 22, 24);

  return (
    <AbsoluteFill
      style={{
        opacity,
        background:
          "radial-gradient(circle at 50% 44%, rgba(216,58,47,0.18), transparent 24%), #030506",
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <div
        style={{
          transform: `translateY(${y}px)`,
          display: "flex",
          flexDirection: "column",
          alignItems: "center",
          gap: 34,
        }}
      >
        <Img
          src={staticFile("icon.png")}
          style={{
            width: 156,
            height: 156,
            objectFit: "contain",
            filter: "drop-shadow(0 0 38px rgba(216,58,47,0.38))",
          }}
        />
        <div
          style={{
            color: ink,
            fontFamily: display,
            fontSize: 94,
            fontWeight: 800,
            lineHeight: 0.92,
            textAlign: "center",
            textTransform: "uppercase",
          }}
        >
          Find plaintext secrets before the agent does.
        </div>
        <div
          style={{
            width: 1130,
            padding: "24px 30px",
            border: `1px solid ${line}`,
            borderLeft: `5px solid ${amber}`,
            background: panelStrong,
            color: ink,
            fontFamily: mono,
            fontSize: 28,
            fontWeight: 800,
            textAlign: "center",
          }}
        >
          {scannerCommand}
        </div>
      </div>
    </AbsoluteFill>
  );
};

export const ScannerOneLinerComposition: React.FC = () => {
  const frame = useCurrentFrame();

  return (
    <AbsoluteFill style={{ background: black }}>
      <Background />
      <Brand />
      <MessageCard
        start={0}
        end={168}
        kicker="One command preflight"
        title="Scan before the agent starts"
        body="A lightweight curl one-liner checks local plaintext credential exposure before an AI run gets filesystem context."
      />
      <MessageCard
        start={142}
        end={334}
        kicker="Sandboxed by default"
        title="Downloads. Unpacks. Runs offline."
        body="The wrapper denies writes and network access, then runs package-specific isotope detectors against common secret paths."
        accent={blue}
      />
      <MessageCard
        start={308}
        end={508}
        kicker="Human-readable findings"
        title="Plaintext secrets become visible"
        body="The scanner reports likely exposure without printing the secret value itself. Netrc files, package tokens, and unencrypted keys stand out fast."
        accent={red}
      />
      <MessageCard
        start={486}
        end={650}
        kicker="Fix the exposure"
        title="Move secrets into Automic Vault"
        body="Install Automic Vault after the scan to move supported credentials out of plaintext and inject them only into trusted tools."
        accent={green}
      />
      <TerminalPanel />
      <CommandStrip />
      {frame >= 572 ? <EndCard /> : null}
    </AbsoluteFill>
  );
};
