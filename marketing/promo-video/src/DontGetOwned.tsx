import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  interpolateColors,
  staticFile,
  useCurrentFrame,
} from "remotion";

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

const introDuration = sec(2.4);
const detectStart = introDuration;
const detectDuration = sec(6.1);
const hardenStart = detectStart + detectDuration;
const hardenDuration = sec(6.5);
const ownedStart = hardenStart + hardenDuration;
const ownedDuration = sec(7.6);
const closeStart = ownedStart + ownedDuration;
const closeDuration = sec(3.1);

export const dontGetOwnedDurationInFrames = closeStart + closeDuration;

const black = "#030506";
const nearBlack = "#071014";
const red = "#d83a2f";
const redHot = "#ff5348";
const green = "#6bffb0";
const amber = "#ffb347";
const blue = "#6aa9ff";
const ink = "#f2e1b8";
const inkMuted = "#b89b73";
const line = "rgba(242, 225, 184, 0.32)";
const lineFaint = "rgba(242, 225, 184, 0.16)";
const display =
  '"Barlow Condensed", "Arial Narrow", Impact, ui-sans-serif, system-ui, sans-serif';
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';

const clamp = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const easeOut = Easing.bezier(0.16, 1, 0.3, 1);
const easeIn = Easing.bezier(0.7, 0, 0.84, 0);

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], {
    ...clamp,
    easing: easeOut,
  });

const fadeOut = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [1, 0], {
    ...clamp,
    easing: easeIn,
  });

const softY = (frame: number, start: number, end: number, from: number, to = 0) =>
  interpolate(frame, [start, end], [from, to], {
    ...clamp,
    easing: easeOut,
  });

const pop = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, Math.floor((start + end) / 2), end], [0.92, 1.08, 1], {
    ...clamp,
    easing: easeOut,
  });

const Background: React.FC<{ mode: "detect" | "harden" | "owned" }> = ({ mode }) => {
  const frame = useCurrentFrame();
  const drift = interpolate(frame, [0, dontGetOwnedDurationInFrames], [0, 1], clamp);
  const accent = mode === "harden" ? green : mode === "owned" ? amber : red;
  const secondary = mode === "harden" ? blue : mode === "owned" ? redHot : amber;
  const slowX = interpolate(drift, [0, 1], [-72, 58], clamp);
  const slowY = interpolate(drift, [0, 1], [36, -44], clamp);
  const pulse = interpolate(frame % 48, [0, 18, 48], [0.64, 1, 0.64], clamp);

  return (
    <AbsoluteFill style={{ background: black, overflow: "hidden" }}>
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(115deg, #030506 0%, #0a0d10 48%, #071014 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          inset: -220,
          opacity: 0.58 * pulse,
          filter: "blur(64px)",
          transform: `translate(${slowX}px, ${slowY}px) rotate(-8deg)`,
          background: `radial-gradient(circle at 22% 26%, ${accent}44, transparent 21%), radial-gradient(circle at 82% 62%, ${secondary}36, transparent 24%), linear-gradient(120deg, transparent 0%, ${accent}20 46%, transparent 74%)`,
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.2,
          transform: `translate(${slowX * 0.18}px, ${slowY * 0.16}px)`,
          background:
            "repeating-linear-gradient(0deg, rgba(242,225,184,0.034) 0, rgba(242,225,184,0.034) 1px, transparent 1px, transparent 8px), repeating-linear-gradient(90deg, rgba(242,225,184,0.018) 0, rgba(242,225,184,0.018) 1px, transparent 1px, transparent 80px)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(circle at center, transparent 40%, rgba(0,0,0,0.7) 82%), linear-gradient(180deg, rgba(0,0,0,0.04), rgba(0,0,0,0.5))",
        }}
      />
    </AbsoluteFill>
  );
};

const Brand: React.FC<{ opacity?: number }> = ({ opacity = 1 }) => (
  <div
    style={{
      position: "absolute",
      left: 70,
      top: 54,
      display: "flex",
      alignItems: "center",
      gap: 18,
      opacity,
    }}
  >
    <Img
      src={staticFile("icon.png")}
      style={{
        width: 66,
        height: 66,
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

const StepLabel: React.FC<{
  local: number;
  step: string;
  verb: string;
  color: string;
  x?: number;
  y?: number;
}> = ({ local, step, verb, color, x = 84, y = 194 }) => {
  const opacity = fade(local, 0, 16);
  const translate = softY(local, 0, 18, 24);
  const scale = pop(local, 0, 18);

  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        opacity,
        transform: `translateY(${translate}px) scale(${scale})`,
        transformOrigin: "left center",
      }}
    >
      <div
        style={{
          color,
          fontFamily: mono,
          fontSize: 30,
          fontWeight: 850,
          letterSpacing: 0,
          textTransform: "uppercase",
        }}
      >
        {step}
      </div>
      <div
        style={{
          marginTop: 20,
          color: ink,
          fontFamily: display,
          fontSize: 172,
          fontWeight: 850,
          letterSpacing: 0,
          lineHeight: 0.82,
          textTransform: "uppercase",
          textShadow: `0 0 42px ${color}44, 0 18px 36px rgba(0,0,0,0.6)`,
        }}
      >
        {verb}
      </div>
    </div>
  );
};

const FlashWord: React.FC<{
  local: number;
  start: number;
  text: string;
  color: string;
  y: number;
  size?: number;
}> = ({ local, start, text, color, y, size = 58 }) => {
  const opacity =
    fade(local, start, start + 5) * fadeOut(local, start + 18, start + 26);
  const xJitter = local >= start && local < start + 26 ? ((local * 29) % 13) - 6 : 0;

  return (
    <div
      style={{
        position: "absolute",
        left: 40 + xJitter,
        top: y,
        width: 780,
        opacity,
        color,
        fontFamily: mono,
        fontSize: size,
        fontWeight: 900,
        letterSpacing: 0,
        lineHeight: 1,
        textTransform: "uppercase",
        textShadow: `0 0 28px ${color}88`,
      }}
    >
      {text}
    </div>
  );
};

const HazardPanel: React.FC<{ local: number }> = ({ local }) => {
  const hazards = [
    { at: 42, text: "HAZARD: postinstall beacon", y: 196 },
    { at: 64, text: "DETECTED: npm token in env", y: 306 },
    { at: 86, text: "ALERT: new maintainer publish", y: 416 },
    { at: 108, text: "TRACE: C2 domain requested", y: 526 },
    { at: 130, text: "FINDING: credential file read", y: 636 },
  ];
  const sweep = interpolate(local % 42, [0, 42], [-120, 920], clamp);
  const pulse = interpolate(local % 18, [0, 9, 18], [0.48, 1, 0.48], clamp);

  return (
    <div
      style={{
        position: "absolute",
        right: 92,
        top: 150,
        width: 864,
        height: 702,
        borderRadius: 8,
        overflow: "hidden",
        border: `1px solid ${line}`,
        background: "rgba(6, 10, 12, 0.72)",
        boxShadow: "0 34px 110px rgba(0,0,0,0.46)",
      }}
    >
      <div
        style={{
          position: "absolute",
          inset: 0,
          opacity: 0.3,
          background:
            "repeating-linear-gradient(180deg, rgba(216,58,47,0.18) 0, rgba(216,58,47,0.18) 1px, transparent 1px, transparent 22px)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: sweep,
          top: 0,
          width: 88,
          height: "100%",
          opacity: 0.74,
          background:
            "linear-gradient(90deg, transparent, rgba(255,83,72,0.58), transparent)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 34,
          top: 30,
          color: redHot,
          opacity: pulse,
          fontFamily: mono,
          fontSize: 24,
          fontWeight: 900,
          letterSpacing: 0,
        }}
      >
        AUTOMIC VAULT DETECTOR STREAM
      </div>
      {hazards.map((hazard) => (
        <FlashWord
          key={hazard.text}
          local={local}
          start={hazard.at}
          text={hazard.text}
          color={redHot}
          y={hazard.y - 150}
        />
      ))}
      {hazards.map((hazard, index) => (
        <div
          key={`${hazard.text}-ghost`}
          style={{
            position: "absolute",
            left: 38,
            top: 126 + index * 102,
            width: 784,
            height: 66,
            opacity: local > hazard.at ? 0.26 : 0.08,
            borderLeft: `5px solid ${red}`,
            background:
              "linear-gradient(90deg, rgba(216,58,47,0.24), rgba(216,58,47,0.04))",
          }}
        />
      ))}
    </div>
  );
};

const DetectScene: React.FC = () => {
  const local = useCurrentFrame();
  const exit = fadeOut(local, detectDuration - 18, detectDuration);

  return (
    <AbsoluteFill style={{ opacity: exit }}>
      <Background mode="detect" />
      <Brand />
      <StepLabel local={local} step="Step 1" verb="Detect" color={redHot} />
      <HazardPanel local={local} />
      <div
        style={{
          position: "absolute",
          left: 92,
          bottom: 118,
          width: 690,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 750,
          lineHeight: 1.35,
        }}
      >
        Package-specific detectors. Secret checks. Behavior that smells wrong
        before it gets a shell.
      </div>
    </AbsoluteFill>
  );
};

const Shield: React.FC<{ local: number; start: number; label: string; x: number; y: number }> = ({
  local,
  start,
  label,
  x,
  y,
}) => {
  const opacity = fade(local, start, start + 14);
  const scale = pop(local, start, start + 18);
  const glow = interpolate(local % 32, [0, 16, 32], [0.42, 0.85, 0.42], clamp);

  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: 242,
        height: 176,
        opacity,
        transform: `scale(${scale})`,
      }}
    >
      <svg
        width="104"
        height="120"
        viewBox="0 0 104 120"
        style={{
          position: "absolute",
          left: 69,
          top: 0,
          filter: `drop-shadow(0 0 ${28 * glow}px rgba(107,255,176,0.56))`,
        }}
      >
        <path
          d="M52 6 L91 22 V53 C91 82 73 104 52 114 C31 104 13 82 13 53 V22 Z"
          fill="rgba(107,255,176,0.16)"
          stroke={green}
          strokeWidth="5"
          strokeLinejoin="round"
        />
        <path
          d="M31 58 L45 72 L75 39"
          fill="none"
          stroke={green}
          strokeWidth="8"
          strokeLinecap="round"
          strokeLinejoin="round"
        />
      </svg>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 0,
          height: 54,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 8,
          color: green,
          background: "rgba(6, 18, 14, 0.82)",
          border: "1px solid rgba(107,255,176,0.38)",
          fontFamily: mono,
          fontSize: 23,
          fontWeight: 900,
          letterSpacing: 0,
          boxShadow: "0 18px 40px rgba(0,0,0,0.34)",
        }}
      >
        {label}
      </div>
    </div>
  );
};

const HardenScene: React.FC = () => {
  const local = useCurrentFrame();
  const exit = fadeOut(local, hardenDuration - 18, hardenDuration);
  const packages = [
    "openssl@3",
    "awscli",
    "node",
    "gh",
    "python@3.13",
    "docker",
    "uv",
    "ffmpeg",
    "git",
  ];

  return (
    <AbsoluteFill style={{ opacity: exit }}>
      <Background mode="harden" />
      <Brand />
      <StepLabel local={local} step="Step 2" verb="Harden" color={green} />
      <div
        style={{
          position: "absolute",
          left: 98,
          bottom: 126,
          width: 660,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 750,
          lineHeight: 1.35,
        }}
      >
        Lock risky install behavior behind reviewable package rules. Agents can
        move fast without inheriting blind trust.
      </div>
      {packages.map((pkg, index) => (
        <Shield
          key={pkg}
          local={local}
          start={42 + index * 10}
          label={pkg}
          x={850 + (index % 3) * 286}
          y={142 + Math.floor(index / 3) * 216}
        />
      ))}
      <div
        style={{
          position: "absolute",
          right: 102,
          bottom: 78,
          color: green,
          fontFamily: mono,
          fontSize: 28,
          fontWeight: 900,
          letterSpacing: 0,
          textTransform: "uppercase",
          opacity: fade(local, 118, 138),
        }}
      >
        approved behaviors only
      </div>
    </AbsoluteFill>
  );
};

const incidentPackages = [
  { name: "node-ipc", detail: "9.1.6 / 9.2.3 / 12.0.1", tone: redHot },
  { name: "durabletask", detail: "1.4.1 - 1.4.3", tone: amber },
  { name: "axios", detail: "1.14.1 / 0.30.4", tone: redHot },
  { name: "plain-crypto-js", detail: "4.2.1", tone: amber },
  { name: "litellm", detail: "1.82.7 / 1.82.8", tone: redHot },
  { name: "telnyx", detail: "PyPI compromise", tone: amber },
  { name: "@tanstack/*", detail: "Mini Shai-Hulud", tone: redHot },
  { name: "@mistralai/*", detail: "campaign wave", tone: amber },
  { name: "UiPath packages", detail: "campaign wave", tone: amber },
  { name: "@antv/g2", detail: "May 19 wave", tone: redHot },
  { name: "@antv/g6", detail: "May 19 wave", tone: redHot },
  { name: "echarts-for-react", detail: "affected", tone: amber },
  { name: "size-sensor", detail: "affected", tone: amber },
  { name: "timeago.js", detail: "affected", tone: amber },
  { name: "@antv/x6", detail: "affected", tone: redHot },
  { name: "@antv/l7", detail: "affected", tone: redHot },
];

const IncidentTile: React.FC<{
  local: number;
  index: number;
  name: string;
  detail: string;
  tone: string;
}> = ({ local, index, name, detail, tone }) => {
  const row = Math.floor(index / 4);
  const column = index % 4;
  const start = 70 + index * 5;
  const opacity = fade(local, start, start + 8);
  const y = softY(local, start, start + 10, 20);
  const pulse = interpolate((local + index * 7) % 22, [0, 11, 22], [0.38, 0.94, 0.38], clamp);

  return (
    <div
      style={{
        position: "absolute",
        left: 104 + column * 428,
        top: 340 + row * 132,
        width: 372,
        height: 96,
        opacity,
        transform: `translateY(${y}px)`,
        borderRadius: 8,
        overflow: "hidden",
        border: `1px solid ${tone}66`,
        background: `linear-gradient(90deg, ${tone}22, rgba(12,18,22,0.84))`,
        boxShadow: `0 0 ${34 * pulse}px ${tone}44, 0 20px 46px rgba(0,0,0,0.42)`,
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 18,
          top: 16,
          color: ink,
          fontFamily: mono,
          fontSize: 25,
          fontWeight: 950,
          letterSpacing: 0,
          lineHeight: 1,
        }}
      >
        {name}
      </div>
      <div
        style={{
          position: "absolute",
          left: 18,
          top: 54,
          color: tone,
          fontFamily: mono,
          fontSize: 18,
          fontWeight: 850,
          letterSpacing: 0,
          textTransform: "uppercase",
        }}
      >
        {detail}
      </div>
    </div>
  );
};

const OwnedScene: React.FC = () => {
  const local = useCurrentFrame();
  const titleColor = interpolateColors(
    local % 18,
    [0, 9, 18],
    [ink, redHot, ink],
  );
  const exit = fadeOut(local, ownedDuration - 18, ownedDuration);

  return (
    <AbsoluteFill style={{ opacity: exit }}>
      <Background mode="owned" />
      <Brand opacity={0.82} />
      <div
        style={{
          position: "absolute",
          left: 90,
          top: 150,
          color: amber,
          fontFamily: mono,
          fontSize: 31,
          fontWeight: 900,
          letterSpacing: 0,
          textTransform: "uppercase",
          opacity: fade(local, 0, 16),
        }}
      >
        Step 3
      </div>
      <div
        style={{
          position: "absolute",
          left: 86,
          top: 204,
          color: titleColor,
          fontFamily: display,
          fontSize: 150,
          fontWeight: 900,
          letterSpacing: 0,
          lineHeight: 0.86,
          textTransform: "uppercase",
          textShadow: "0 0 44px rgba(216,58,47,0.48), 0 20px 40px rgba(0,0,0,0.62)",
          opacity: fade(local, 4, 20),
        }}
      >
        Don&apos;t get owned.
      </div>
      <div
        style={{
          position: "absolute",
          right: 100,
          top: 156,
          width: 520,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 29,
          fontWeight: 750,
          lineHeight: 1.35,
          textAlign: "right",
          opacity: fade(local, 24, 42),
        }}
      >
        These were real package incidents. Your terminal only said install
        succeeded.
      </div>
      {incidentPackages.map((pkg, index) => (
        <IncidentTile
          key={`${pkg.name}-${pkg.detail}`}
          local={local}
          index={index}
          name={pkg.name}
          detail={pkg.detail}
          tone={pkg.tone}
        />
      ))}
      <div
        style={{
          position: "absolute",
          left: 104,
          bottom: 76,
          color: redHot,
          fontFamily: mono,
          fontSize: 28,
          fontWeight: 900,
          letterSpacing: 0,
          textTransform: "uppercase",
          opacity: fade(local, 142, 160),
        }}
      >
        assume compromise. rotate secrets. harden before the next wave.
      </div>
    </AbsoluteFill>
  );
};

const IntroScene: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity = fade(frame, 0, 18) * fadeOut(frame, introDuration - 16, introDuration);
  const y = softY(frame, 0, 20, 20);

  return (
    <AbsoluteFill style={{ opacity }}>
      <Background mode="detect" />
      <Brand />
      <div
        style={{
          position: "absolute",
          left: 92,
          top: 270,
          transform: `translateY(${y}px)`,
          color: ink,
          fontFamily: display,
          fontSize: 154,
          fontWeight: 900,
          letterSpacing: 0,
          lineHeight: 0.86,
          textTransform: "uppercase",
          textShadow: "0 20px 46px rgba(0,0,0,0.62)",
        }}
      >
        Packages move fast.
        <br />
        Attackers move faster.
      </div>
      <div
        style={{
          position: "absolute",
          left: 100,
          bottom: 144,
          color: redHot,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 900,
          letterSpacing: 0,
          textTransform: "uppercase",
        }}
      >
        detect / harden / don&apos;t get owned
      </div>
    </AbsoluteFill>
  );
};

const CloseScene: React.FC = () => {
  const local = useCurrentFrame();
  const opacity = fade(local, 0, 20);
  const color = interpolateColors(local % 54, [0, 27, 54], [redHot, green, redHot]);

  return (
    <AbsoluteFill style={{ opacity }}>
      <Background mode="harden" />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 252,
          display: "flex",
          justifyContent: "center",
        }}
      >
        <Img
          src={staticFile("icon.png")}
          style={{
            width: 150,
            height: 150,
            objectFit: "contain",
            filter: `drop-shadow(0 0 40px ${color}55)`,
          }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 436,
          color: ink,
          fontFamily: display,
          fontSize: 122,
          fontWeight: 900,
          letterSpacing: 0,
          lineHeight: 0.92,
          textAlign: "center",
          textTransform: "uppercase",
        }}
      >
        Stop installing blind.
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 585,
          display: "flex",
          justifyContent: "center",
        }}
      >
        <Img
          src={staticFile("site-wordmark.webp")}
          style={{
            width: 358,
            height: "auto",
            objectFit: "contain",
            filter: "drop-shadow(0 18px 28px rgba(0,0,0,0.48))",
          }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 142,
          color,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 900,
          letterSpacing: 0,
          textAlign: "center",
          textTransform: "uppercase",
        }}
      >
        step 1 detect. step 2 harden. step 3 don&apos;t get owned.
      </div>
    </AbsoluteFill>
  );
};

export const DontGetOwnedComposition: React.FC = () => {
  return (
    <AbsoluteFill style={{ background: nearBlack }}>
      <Sequence durationInFrames={introDuration}>
        <IntroScene />
      </Sequence>
      <Sequence from={detectStart} durationInFrames={detectDuration}>
        <DetectScene />
      </Sequence>
      <Sequence from={hardenStart} durationInFrames={hardenDuration}>
        <HardenScene />
      </Sequence>
      <Sequence from={ownedStart} durationInFrames={ownedDuration}>
        <OwnedScene />
      </Sequence>
      <Sequence from={closeStart} durationInFrames={closeDuration}>
        <CloseScene />
      </Sequence>
      <AbsoluteFill
        style={{
          pointerEvents: "none",
          border: `1px solid ${lineFaint}`,
          boxShadow: "inset 0 0 90px rgba(0,0,0,0.34)",
        }}
      />
    </AbsoluteFill>
  );
};
