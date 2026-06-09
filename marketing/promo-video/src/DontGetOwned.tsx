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
    { at: 42, text: "POSTINSTALL", y: 196 },
    { at: 64, text: "NPM TOKEN", y: 306 },
    { at: 86, text: "NEW MAINTAINER", y: 416 },
    { at: 108, text: "C2 REQUEST", y: 526 },
    { at: 130, text: "SECRET READ", y: 636 },
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
        DETECTOR STREAM
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
  const opacity = fade(local, start, start + 4) * fadeOut(local, start + 10, start + 14);
  const scale = pop(local, start, start + 18);
  const glow = interpolate(local % 32, [0, 16, 32], [0.42, 0.85, 0.42], clamp);

  return (
    <div
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: 430,
        height: 300,
        opacity,
        transform: `scale(${scale})`,
      }}
    >
      <svg
        width="190"
        height="220"
        viewBox="0 0 104 120"
        style={{
          position: "absolute",
          left: 120,
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
          height: 74,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          borderRadius: 8,
          color: green,
          background: "rgba(6, 18, 14, 0.82)",
          border: "1px solid rgba(107,255,176,0.38)",
          fontFamily: mono,
          fontSize: 34,
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
      {packages.map((pkg, index) => (
        <Shield
          key={pkg}
          local={local}
          start={38 + index * 17}
          label={pkg}
          x={1038 + (index % 3) * 42}
          y={266 + (index % 2) * 42}
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
        approved only
      </div>
    </AbsoluteFill>
  );
};

const incidentPackages = [
  { name: "node-ipc", tone: redHot },
  { name: "durabletask", tone: amber },
  { name: "axios", tone: redHot },
  { name: "plain-crypto-js", tone: amber },
  { name: "litellm", tone: redHot },
  { name: "telnyx", tone: amber },
  { name: "@tanstack/*", tone: redHot },
  { name: "@mistralai/*", tone: amber },
  { name: "UiPath", tone: amber },
  { name: "@antv/g2", tone: redHot },
  { name: "@antv/g6", tone: redHot },
  { name: "echarts-for-react", tone: amber },
  { name: "size-sensor", tone: amber },
  { name: "timeago.js", tone: amber },
  { name: "@antv/x6", tone: redHot },
  { name: "@antv/l7", tone: redHot },
];

const IncidentFlash: React.FC<{
  local: number;
  index: number;
  name: string;
  tone: string;
}> = ({ local, index, name, tone }) => {
  const start = 54 + index * 11;
  const end = start + 18;
  const visible = local >= start && local < end;
  const opacity = fade(local, start, start + 3) * fadeOut(local, end - 4, end);
  const scale = interpolate(local, [start, start + 5, end], [0.82, 1.08, 0.98], clamp);
  const x = visible ? ((local * 37 + index * 11) % 24) - 12 : 0;
  const rotate = visible ? (((local + index) % 7) - 3) * 0.5 : 0;

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top: 470,
        opacity,
        transform: `translateX(${x}px) rotate(${rotate}deg) scale(${scale})`,
          color: tone,
          fontFamily: display,
          fontSize: 132,
          fontWeight: 900,
          letterSpacing: 0,
          lineHeight: 0.86,
          textAlign: "center",
          textTransform: "uppercase",
          textShadow: `0 0 38px ${tone}88, 0 20px 40px rgba(0,0,0,0.62)`,
        }}
      >
        {name}
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
      {incidentPackages.map((pkg, index) => (
        <IncidentFlash
          key={pkg.name}
          local={local}
          index={index}
          name={pkg.name}
          tone={pkg.tone}
        />
      ))}
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
        Detect.
        <br />
        Harden.
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
        don&apos;t get owned
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
