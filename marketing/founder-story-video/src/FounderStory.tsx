import { AbsoluteFill, interpolate, useCurrentFrame } from "remotion";

const black = "#020203";
const red = "#c0221d";
const redDim = "rgba(192, 34, 29, 0.22)";
const redFaint = "rgba(192, 34, 29, 0.1)";
const text = "#d7c7a1";
const mono =
  '"IBM Plex Mono", "Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, monospace';

export const founderStoryDurationInFrames = 360;

const clamp = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], clamp);

const SceneText: React.FC<{
  lines: string[];
  start: number;
  end: number;
  size?: number;
  y?: number;
}> = ({ lines, start, end, size = 58, y = 0 }) => {
  const frame = useCurrentFrame();
  const opacity =
    fade(frame, start + 10, start + 34) *
    interpolate(frame, [end - 24, end], [1, 0], clamp);

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity,
        transform: `translateY(${y}px)`,
      }}
    >
      <div
        style={{
          color: text,
          fontFamily: mono,
          fontSize: size,
          fontWeight: 600,
          letterSpacing: 0,
          lineHeight: 1.28,
          textAlign: "center",
          textShadow: "0 0 18px rgba(192,34,29,0.28), 0 24px 46px rgba(0,0,0,0.72)",
        }}
      >
        {lines.map((line) => (
          <div key={line}>{line}</div>
        ))}
      </div>
    </AbsoluteFill>
  );
};

const Background: React.FC = () => {
  const frame = useCurrentFrame();
  const gridY = interpolate(frame, [0, 360], [0, -86], clamp);
  const noiseA = frame % 2 === 0 ? 0.055 : 0.035;
  const noiseB = frame % 5 === 0 ? 0.04 : 0.02;

  return (
    <AbsoluteFill style={{ background: black }}>
      <AbsoluteFill
        style={{
          opacity: 0.82,
          background:
            "radial-gradient(circle at 50% 42%, rgba(192,34,29,0.12), transparent 30%), linear-gradient(180deg, #030304 0%, #090505 52%, #020203 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.34,
          backgroundImage:
            "linear-gradient(rgba(192,34,29,0.22) 1px, transparent 1px), linear-gradient(90deg, rgba(192,34,29,0.18) 1px, transparent 1px)",
          backgroundSize: "76px 76px",
          transform: `translateY(${gridY}px) perspective(780px) rotateX(58deg) scale(1.6)`,
          transformOrigin: "50% 72%",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18,
          background:
            "linear-gradient(90deg, rgba(192,34,29,0.1), transparent 22%, transparent 78%, rgba(192,34,29,0.08))",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: noiseA,
          background:
            "repeating-radial-gradient(circle at 17% 23%, rgba(255,255,255,0.35) 0, rgba(255,255,255,0.35) 1px, transparent 1px, transparent 5px)",
          mixBlendMode: "screen",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: noiseB,
          background:
            "repeating-linear-gradient(90deg, transparent 0, transparent 9px, rgba(255,255,255,0.12) 10px, transparent 11px)",
          mixBlendMode: "screen",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18,
          background:
            "repeating-linear-gradient(180deg, rgba(255,255,255,0.14) 0, rgba(255,255,255,0.14) 1px, transparent 1px, transparent 5px)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(circle at center, transparent 38%, rgba(0,0,0,0.72) 86%)",
        }}
      />
    </AbsoluteFill>
  );
};

const ScanningBeam: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity =
    frame >= 90 && frame < 180
      ? fade(frame, 90, 104) * interpolate(frame, [166, 180], [1, 0], clamp)
      : 0;
  const left = interpolate(frame, [96, 174], [-12, 101], clamp);

  return (
    <AbsoluteFill style={{ opacity }}>
      <div
        style={{
          position: "absolute",
          left: `${left}%`,
          top: 0,
          width: 5,
          height: "100%",
          background:
            "linear-gradient(90deg, transparent, rgba(192,34,29,0.58), rgba(255,214,190,0.22), transparent)",
          boxShadow: "0 0 24px rgba(192,34,29,0.38)",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: `${left - 2.2}%`,
          top: 0,
          width: "4.4%",
          height: "100%",
          background:
            "linear-gradient(90deg, transparent, rgba(192,34,29,0.08), transparent)",
        }}
      />
    </AbsoluteFill>
  );
};

const RadarRing: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity =
    frame >= 180 && frame < 300
      ? fade(frame, 184, 204) * interpolate(frame, [278, 300], [1, 0], clamp)
      : 0;
  const size = interpolate(frame, [180, 300], [140, 760], clamp);
  const ringOpacity = interpolate(frame, [180, 300], [0.38, 0], clamp);

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity,
      }}
    >
      <div
        style={{
          width: size,
          height: size,
          border: `1px solid rgba(192,34,29,${ringOpacity})`,
          borderRadius: "50%",
          boxShadow: `0 0 42px rgba(192,34,29,${ringOpacity * 0.38})`,
        }}
      />
    </AbsoluteFill>
  );
};

const FinalTitle: React.FC = () => {
  const frame = useCurrentFrame();
  const visible = frame >= 300;
  const flicker = visible && frame < 336 && frame % 9 === 0 ? 0.72 : 1;

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: visible ? flicker : 0,
      }}
    >
      <div
        style={{
          color: red,
          fontFamily: mono,
          fontSize: 86,
          fontWeight: 700,
          letterSpacing: 6,
          lineHeight: 1,
          textAlign: "center",
          textShadow: "0 0 16px rgba(192,34,29,0.34), 0 28px 52px rgba(0,0,0,0.78)",
        }}
      >
        AUTOMIC VAULT
      </div>
    </AbsoluteFill>
  );
};

export const FounderStory: React.FC = () => {
  const frame = useCurrentFrame();
  const zoom = frame >= 180 && frame < 300 ? interpolate(frame, [180, 300], [1, 1.035], clamp) : 1;

  return (
    <AbsoluteFill style={{ background: black }}>
      <AbsoluteFill style={{ transform: `scale(${zoom})` }}>
        <Background />
        <RadarRing />
        <SceneText lines={["I built Homebrew.", "Right as Web 2 began."]} start={0} end={90} />
        <SceneText
          lines={["Something new is starting."]}
          start={90}
          end={180}
          size={62}
        />
        <SceneText
          lines={["So I’m building again."]}
          start={180}
          end={300}
          size={66}
        />
      </AbsoluteFill>
      <ScanningBeam />
      <FinalTitle />
      <AbsoluteFill
        style={{
          border: `1px solid ${redFaint}`,
          inset: 48,
          width: "auto",
          height: "auto",
          boxShadow: `inset 0 0 48px ${redDim}`,
        }}
      />
    </AbsoluteFill>
  );
};
