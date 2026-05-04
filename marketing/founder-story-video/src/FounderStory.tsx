import {
  AbsoluteFill,
  Easing,
  Img,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";

const black = "#020303";
const ink = "#e6d7b2";
const inkMuted = "#a88f6d";
const green = "#7cffbc";
const red = "#d83a2f";
const line = "rgba(230, 215, 178, 0.14)";
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';
const sans =
  '"Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';

const fps = 30;
const sec = (value: number) => Math.round(value * fps);
export const founderStoryDurationInFrames = sec(12.65);

const clamp = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const softEase = Easing.bezier(0.16, 1, 0.3, 1);
const hardEase = Easing.bezier(0.08, 0.82, 0.17, 1);

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], {
    ...clamp,
    easing: softEase,
  });

const typedText = (text: string, frame: number, start: number, duration: number) => {
  if (frame < start) {
    return "";
  }

  const progress = interpolate(frame, [start, start + duration], [0, 1], clamp);
  return text.slice(0, Math.ceil(text.length * progress));
};

const BlackField: React.FC<{ redHaze?: number }> = ({ redHaze = 0 }) => (
  <AbsoluteFill style={{ background: black }}>
    <AbsoluteFill
      style={{
        opacity: 0.18,
        background:
          "radial-gradient(circle at 50% 42%, rgba(230,215,178,0.08), transparent 28%), linear-gradient(180deg, rgba(255,255,255,0.025), transparent 44%)",
      }}
    />
    <AbsoluteFill
      style={{
        opacity: 0.1,
        background:
          "repeating-linear-gradient(0deg, rgba(230,215,178,0.06) 0, rgba(230,215,178,0.06) 1px, transparent 1px, transparent 7px)",
      }}
    />
    <AbsoluteFill
      style={{
        opacity: redHaze,
        background:
          "radial-gradient(circle at 70% 46%, rgba(216,58,47,0.36), transparent 22%), linear-gradient(90deg, transparent, rgba(216,58,47,0.08), transparent)",
      }}
    />
    <AbsoluteFill
      style={{
        background:
          "radial-gradient(circle at center, transparent 36%, rgba(0,0,0,0.76) 86%)",
      }}
    />
  </AbsoluteFill>
);

const TerminalFrame: React.FC<{ start: number; end: number }> = ({ start, end }) => {
  const frame = useCurrentFrame();
  const opacity =
    fade(frame, start, start + 14) * interpolate(frame, [end - 18, end], [1, 0], clamp);
  const scale = interpolate(frame, [start, end], [0.992, 1.006], clamp);

  return (
    <AbsoluteFill style={{ opacity: opacity * 0.78, transform: `scale(${scale})` }}>
      <div
        style={{
          position: "absolute",
          left: 96,
          top: 284,
          width: 1210,
          height: 324,
          border: `1px solid ${line}`,
          background:
            "linear-gradient(180deg, rgba(124,255,188,0.025), transparent 40%), rgba(0,0,0,0.18)",
          boxShadow: "0 34px 80px rgba(0,0,0,0.42), inset 0 0 54px rgba(124,255,188,0.025)",
        }}
      />
    </AbsoluteFill>
  );
};

const Cursor: React.FC<{ visible: boolean; height?: number }> = ({
  visible,
  height = 58,
}) => {
  const frame = useCurrentFrame();

  return (
    <span
      style={{
        display: "inline-block",
        width: 9,
        height,
        marginLeft: 14,
        borderRadius: 2,
        background: green,
        opacity: visible && frame % 30 < 15 ? 1 : 0,
        boxShadow: "0 0 22px rgba(124,255,188,0.42)",
      }}
    />
  );
};

const TerminalLine: React.FC<{
  text: string;
  start: number;
  typeDuration: number;
  y: number;
  holdUntil: number;
  size?: number;
  muted?: boolean;
  cursorUntil?: number;
}> = ({
  text,
  start,
  typeDuration,
  y,
  holdUntil,
  size = 54,
  muted = false,
  cursorUntil,
}) => {
  const frame = useCurrentFrame();
  const typed = typedText(text, frame, start, typeDuration);
  const opacityIn = fade(frame, start - 6, start + 10);
  const opacityOut = interpolate(frame, [holdUntil - 18, holdUntil], [1, 0], clamp);
  const yIn = interpolate(frame, [start - 8, start + 14], [12, 0], {
    ...clamp,
    easing: softEase,
  });
  const cursorVisible = frame >= start && frame < (cursorUntil ?? start + typeDuration + 24);

  return (
    <div
      style={{
        position: "absolute",
        left: 150,
        top: y,
        display: "flex",
        alignItems: "center",
        height: size * 1.35,
        color: muted ? inkMuted : green,
        fontFamily: mono,
        fontSize: size,
        fontWeight: 700,
        letterSpacing: 0,
        lineHeight: 1.18,
        opacity: opacityIn * opacityOut,
        transform: `translateY(${yIn}px)`,
        textShadow: muted ? "none" : "0 0 18px rgba(124,255,188,0.26)",
      }}
    >
      <span>{typed}</span>
      <Cursor visible={cursorVisible} height={size * 1.08} />
    </div>
  );
};

const KineticLine: React.FC<{
  text: string;
  start: number;
  end: number;
  size: number;
  accent?: boolean;
  product?: boolean;
}> = ({ text, start, end, size, accent = false, product = false }) => {
  const frame = useCurrentFrame();
  const inFrames = product ? 16 : 8;
  const outFrames = product ? 18 : 10;
  const opacity =
    fade(frame, start, start + inFrames) *
    interpolate(frame, [end - outFrames, end], [1, 0], clamp);
  const lift = interpolate(frame, [start, start + inFrames], [product ? 36 : 24, 0], {
    ...clamp,
    easing: product ? hardEase : softEase,
  });
  const scale = interpolate(frame, [start, start + inFrames, end], [
    product ? 0.84 : 0.96,
    product ? 1.045 : 1.015,
    product ? 1.018 : 1,
  ], {
    ...clamp,
    easing: product ? hardEase : softEase,
  });

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
          maxWidth: 1540,
          color: accent || product ? red : ink,
          fontFamily: sans,
          fontSize: size,
          fontWeight: product ? 900 : 850,
          letterSpacing: product ? 1.6 : 0,
          lineHeight: 1,
          textAlign: "center",
          transform: `translateY(${lift}px) scale(${scale})`,
          textShadow:
            accent || product
              ? "0 0 38px rgba(216,58,47,0.36), 0 32px 62px rgba(0,0,0,0.76)"
              : "0 26px 48px rgba(0,0,0,0.66)",
        }}
      >
        {text}
      </div>
    </AbsoluteFill>
  );
};

const RocketLogoClose: React.FC<{ start: number; end: number }> = ({ start, end }) => {
  const frame = useCurrentFrame();
  const localFrame = frame - start;
  const opacity =
    fade(frame, start, start + 16) * interpolate(frame, [end - 18, end], [1, 0], clamp);
  const logoOpacity = fade(localFrame, 8, 22);
  const logoY = interpolate(localFrame, [0, 26], [-54, -82], {
    ...clamp,
    easing: hardEase,
  });
  const logoScale = interpolate(localFrame, [0, 26, end - start], [0.86, 1.02, 1], {
    ...clamp,
    easing: hardEase,
  });
  const wordOpacity = fade(localFrame, 24, 44);
  const urlOpacity = fade(localFrame, 44, 64);
  const redFlash = interpolate(localFrame, [10, 20, 34], [0, 1, 0], clamp);

  return (
    <AbsoluteFill style={{ opacity }}>
      <Img
        src={staticFile("rocket-grid.png")}
        style={{
          position: "absolute",
          inset: 0,
          width: "100%",
          height: "100%",
          objectFit: "cover",
          opacity: 0.8,
          filter: "contrast(1.08) saturate(0.84) sepia(0.18) brightness(0.5)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(90deg, rgba(2,3,3,0.92) 0%, rgba(2,3,3,0.5) 50%, rgba(2,3,3,0.92) 100%), radial-gradient(circle at 58% 76%, rgba(216,58,47,0.24), transparent 18%), radial-gradient(circle at center, transparent 0, rgba(2,3,3,0.14) 34%, rgba(2,3,3,0.78) 82%)",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: redFlash * 0.28,
          background:
            "radial-gradient(circle at center, rgba(216,58,47,0.42), transparent 22%)",
        }}
      />
      <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
        <Img
          src={staticFile("icon.png")}
          style={{
            position: "absolute",
            width: 270,
            height: 270,
            objectFit: "contain",
            opacity: logoOpacity,
            transform: `translateY(${logoY}px) scale(${logoScale})`,
            filter:
              "drop-shadow(0 0 32px rgba(216,58,47,0.3)) drop-shadow(0 0 14px rgba(124,255,188,0.14))",
          }}
        />
        <Img
          src={staticFile("wordmark.png")}
          style={{
            position: "absolute",
            top: 538,
            width: 700,
            height: 326,
            objectFit: "contain",
            opacity: wordOpacity,
            filter: "drop-shadow(0 18px 24px rgba(0,0,0,0.62))",
          }}
        />
        <div
          style={{
            position: "absolute",
            bottom: 92,
            color: inkMuted,
            fontFamily: mono,
            fontSize: 34,
            fontWeight: 700,
            letterSpacing: 0,
            opacity: urlOpacity,
            textShadow: "0 16px 28px rgba(0,0,0,0.7)",
          }}
        >
          https://automicvault.com
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};

export const FounderStory: React.FC = () => {
  const frame = useCurrentFrame();
  const t = {
    homebrew: sec(0),
    web: sec(2.25),
    agentsDawn: sec(4.85),
    didItAgain: sec(7.9),
    vault: sec(9.9),
    end: sec(12.65),
  };

  return (
    <AbsoluteFill style={{ background: black }}>
      <BlackField redHaze={frame >= t.vault ? 0.26 : 0} />
      <TerminalFrame start={t.homebrew} end={t.agentsDawn} />
      <TerminalLine
        text="I created Homebrew"
        start={t.homebrew}
        typeDuration={sec(1.2)}
        y={342}
        holdUntil={t.agentsDawn}
        cursorUntil={sec(1.7)}
        size={58}
      />
      <TerminalLine
        text="At the dawn of Web 2.0"
        start={t.web}
        typeDuration={sec(1.25)}
        y={462}
        holdUntil={t.agentsDawn}
        size={54}
        muted
        cursorUntil={sec(4.1)}
      />
      <KineticLine
        text="It's now the dawn of agents"
        start={t.agentsDawn}
        end={t.didItAgain}
        size={92}
      />
      <KineticLine
        text="So I did it again"
        start={t.didItAgain}
        end={t.vault}
        size={112}
        accent
      />
      <RocketLogoClose start={t.vault} end={t.end} />
    </AbsoluteFill>
  );
};
