import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
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
export const founderStoryDurationInFrames = sec(42.6);

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

const BlackField: React.FC<{ haze?: number; redHaze?: number }> = ({
  haze = 0.24,
  redHaze = 0,
}) => (
  <AbsoluteFill style={{ background: black }}>
    <AbsoluteFill
      style={{
        opacity: haze,
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
          "radial-gradient(circle at 70% 46%, rgba(216,58,47,0.3), transparent 22%), linear-gradient(90deg, transparent, rgba(216,58,47,0.08), transparent)",
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

const PulseOverlay: React.FC<{ start: number; end: number; strength?: number }> = ({
  start,
  end,
  strength = 1,
}) => {
  const frame = useCurrentFrame();
  const opacity =
    fade(frame, start, start + 6) *
    interpolate(frame, [end - 8, end], [1, 0], clamp) *
    strength;
  const sweep = interpolate(frame, [start, end], [-24, 24], clamp);

  return (
    <AbsoluteFill
      style={{
        opacity,
        background:
          "radial-gradient(circle at 68% 48%, rgba(216,58,47,0.22), transparent 19%), linear-gradient(90deg, transparent 0%, rgba(216,58,47,0.08) 47%, transparent 62%)",
        transform: `translateX(${sweep}px)`,
      }}
    />
  );
};

const TerminalFrame: React.FC<{ start: number; end: number }> = ({ start, end }) => {
  const frame = useCurrentFrame();
  const opacity = fade(frame, start, start + 14) * interpolate(frame, [end - 18, end], [1, 0], clamp);
  const scale = interpolate(frame, [start, end], [0.992, 1.006], clamp);

  return (
    <AbsoluteFill
      style={{
        opacity: opacity * 0.78,
        transform: `scale(${scale})`,
      }}
    >
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
  size?: number;
  weight?: number;
  muted?: boolean;
  accent?: "red" | "green";
  variant?: "quiet" | "statement" | "impact" | "product" | "question";
  y?: number;
  maxWidth?: number;
}> = ({
  text,
  start,
  end,
  size = 74,
  weight = 800,
  muted = false,
  accent,
  variant = "statement",
  y = 0,
  maxWidth = 1540,
}) => {
  const frame = useCurrentFrame();
  const inFrames = variant === "impact" ? 5 : variant === "product" ? 16 : variant === "quiet" ? 16 : 10;
  const outFrames = variant === "impact" ? 5 : variant === "product" ? 16 : 10;
  const inAmount = fade(frame, start, start + inFrames);
  const outAmount = interpolate(frame, [end - outFrames, end], [1, 0], clamp);
  const lift = interpolate(
    frame,
    [start, start + inFrames],
    [variant === "impact" ? 42 : variant === "quiet" ? 16 : 26, 0],
    {
      ...clamp,
      easing: variant === "impact" ? hardEase : softEase,
    },
  );
  const scale = interpolate(frame, [start, start + inFrames, end], [
    variant === "product" ? 0.86 : variant === "impact" ? 0.92 : 0.982,
    variant === "impact" || variant === "product" ? 1.035 : 1,
    variant === "quiet" ? 1.006 : 1.012,
  ], {
    ...clamp,
    easing: variant === "impact" ? hardEase : softEase,
  });
  const tracking = variant === "product" ? 1.4 : 0;
  const shadow =
    accent === "red" || variant === "product"
      ? "0 0 34px rgba(216,58,47,0.34), 0 30px 58px rgba(0,0,0,0.74)"
      : variant === "quiet"
        ? "0 18px 36px rgba(0,0,0,0.54)"
        : "0 26px 48px rgba(0,0,0,0.66)";

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: inAmount * outAmount,
      }}
    >
      <div
        style={{
          maxWidth,
          color: accent === "red" ? red : accent === "green" ? green : muted ? inkMuted : ink,
          fontFamily: sans,
          fontSize: size,
          fontWeight: weight,
          letterSpacing: tracking,
          lineHeight: 1.08,
          textAlign: "center",
          transform: `translateY(${y + lift}px) scale(${scale})`,
          textShadow: shadow,
        }}
      >
        {text}
      </div>
    </AbsoluteFill>
  );
};

const WordFlashSequence: React.FC<{
  words: string[];
  start: number;
  weights: number[];
  size?: number;
  accentWords?: Record<string, "red" | "green">;
}> = ({ words, start, weights, size = 122, accentWords = {} }) => {
  const frame = useCurrentFrame();
  let cursor = start;
  const items = words.map((word, index) => {
    const duration = weights[index] ?? 7;
    const item = {
      duration,
      start: cursor,
      word,
    };
    cursor += duration;
    return item;
  });

  return (
    <AbsoluteFill>
      {items.map((item) => {
        const end = item.start + item.duration;
        const visible = frame >= item.start && frame < end;
        const flash = visible ? interpolate(frame, [item.start, item.start + 2, end], [0, 1, 0], clamp) : 0;
        const scale = visible
          ? interpolate(frame, [item.start, item.start + 3, end], [0.86, 1.08, 1.018], {
              ...clamp,
              easing: hardEase,
            })
          : 1;

        return (
          <AbsoluteFill
            key={`${item.word}-${item.start}`}
            style={{
              alignItems: "center",
              justifyContent: "center",
              opacity: visible ? 1 : 0,
              background:
                accentWords[item.word.toLowerCase()] === "red"
                  ? `radial-gradient(circle at center, rgba(216,58,47,${0.16 * flash}), transparent 22%)`
                  : undefined,
            }}
          >
            <div
              style={{
                color:
                  accentWords[item.word.toLowerCase()] === "red"
                    ? red
                    : accentWords[item.word.toLowerCase()] === "green"
                      ? green
                      : ink,
                fontFamily: sans,
                fontSize: size,
                fontWeight: 850,
                letterSpacing: 0,
                lineHeight: 1,
                textAlign: "center",
                transform: `scale(${scale})`,
                textShadow:
                  accentWords[item.word.toLowerCase()] === "red"
                    ? "0 0 48px rgba(216,58,47,0.42), 0 34px 68px rgba(0,0,0,0.78)"
                    : "0 34px 68px rgba(0,0,0,0.78)",
              }}
            >
              {item.word}
            </div>
          </AbsoluteFill>
        );
      })}
    </AbsoluteFill>
  );
};

const OpenSourceRunsOn: React.FC<{ start: number; end: number }> = ({ start, end }) => {
  const frame = useCurrentFrame();
  const headingOpacity =
    fade(frame, start, start + 8) * interpolate(frame, [end - 10, end], [1, 0], clamp);
  const firstStart = start + sec(0.7);
  const secondStart = firstStart + sec(2);
  const headingY = interpolate(frame, [start, start + 12], [18, 0], {
    ...clamp,
    easing: softEase,
  });
  const bulletOpacity = (bulletStart: number, bulletEnd: number) =>
    fade(frame, bulletStart, bulletStart + 7) *
    interpolate(frame, [bulletEnd - 8, bulletEnd], [1, 0], clamp);
  const bulletTransform = (bulletStart: number, bulletEnd: number) => {
    const impact = interpolate(frame, [bulletStart, bulletStart + 5, bulletEnd], [0.94, 1.02, 1], {
      ...clamp,
      easing: hardEase,
    });
    const x = interpolate(frame, [bulletStart, bulletStart + 5], [-34, 0], {
      ...clamp,
      easing: hardEase,
    });
    return `translateX(${x}px) scale(${impact})`;
  };

  const bulletStyle: React.CSSProperties = {
    position: "absolute",
    left: 176,
    top: 328,
    maxWidth: 1260,
    color: red,
    fontFamily: sans,
    fontSize: 98,
    fontWeight: 850,
    letterSpacing: 0,
    lineHeight: 1.04,
    textShadow: "0 0 34px rgba(216,58,47,0.3), 0 28px 50px rgba(0,0,0,0.72)",
  };

  return (
    <AbsoluteFill>
      <div
        style={{
          position: "absolute",
          left: 148,
          top: 128,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 44,
          fontWeight: 800,
          letterSpacing: 0,
          opacity: headingOpacity,
          transform: `translateY(${headingY}px)`,
          textShadow: "0 18px 38px rgba(0,0,0,0.7)",
        }}
      >
        OPEN SOURCE RUNS ON
      </div>
      <div
        style={{
          ...bulletStyle,
          opacity: bulletOpacity(firstStart, secondStart),
          transform: bulletTransform(firstStart, secondStart),
        }}
      >
        - Plain Text Secrets.
      </div>
      <div
        style={{
          ...bulletStyle,
          opacity: bulletOpacity(secondStart, end),
          fontSize: 102,
          transform: bulletTransform(secondStart, end),
        }}
      >
        - One liners that can delete prod.
      </div>
    </AbsoluteFill>
  );
};

const SoBuiltIt: React.FC = () => {
  return (
    <WordFlashSequence
      words={["so", "i", "built", "it."]}
      start={0}
      weights={[5, 4, 10, 7]}
      size={178}
      accentWords={{ "it.": "red" }}
    />
  );
};

const Close: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps: videoFps } = useVideoConfig();
  const logoSpring = spring({
    frame: frame - 22,
    fps: videoFps,
    config: { damping: 13, stiffness: 210, mass: 0.6 },
  });
  const logoOpacity = fade(frame, 10, 22);
  const logoY = interpolate(logoSpring, [0, 1], [-210, -74], clamp);
  const logoScale = interpolate(logoSpring, [0, 1], [1.28, 1], clamp);
  const wordOpacity = fade(frame, 58, 86);
  const urlOpacity = fade(frame, 102, 126);
  const flash = interpolate(frame, [20, 22, 28], [0, 1, 0], clamp);

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <Img
        src={staticFile("rocket-grid.png")}
        style={{
          position: "absolute",
          inset: 0,
          width: "100%",
          height: "100%",
          objectFit: "cover",
          opacity: 0.78,
          filter: "contrast(1.08) saturate(0.84) sepia(0.18) brightness(0.5)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(90deg, rgba(2,3,3,0.9) 0%, rgba(2,3,3,0.54) 48%, rgba(2,3,3,0.9) 100%), radial-gradient(circle at 58% 76%, rgba(216,58,47,0.24), transparent 18%), radial-gradient(circle at center, transparent 0, rgba(2,3,3,0.14) 34%, rgba(2,3,3,0.76) 82%)",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: flash * 0.26,
          background:
            "radial-gradient(circle at center, rgba(216,58,47,0.36), transparent 21%)",
        }}
      />
      <Img
        src={staticFile("icon.png")}
        style={{
          position: "absolute",
          width: 286,
          height: 286,
          objectFit: "contain",
          opacity: logoOpacity,
          transform: `translateY(${logoY}px) scale(${logoScale})`,
          filter:
            "drop-shadow(0 0 30px rgba(216,58,47,0.28)) drop-shadow(0 0 14px rgba(124,255,188,0.14))",
        }}
      />
      <Img
        src={staticFile("wordmark.png")}
        style={{
          position: "absolute",
          top: 540,
          width: 720,
          height: 334,
          objectFit: "contain",
          opacity: wordOpacity,
          filter: "drop-shadow(0 18px 24px rgba(0,0,0,0.6))",
        }}
      />
      <div
        style={{
          position: "absolute",
          top: 848,
          color: inkMuted,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 600,
          letterSpacing: 0,
          opacity: urlOpacity,
        }}
      >
        automicvault.com
      </div>
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
    introducing: sec(9.9),
    everythingOpenSource: sec(12.65),
    agentsOpenSource: sec(15.35),
    openSourceRunsOn: sec(17.95),
    blackOne: sec(22.65),
    infrastructure: sec(23.95),
    toolsLayer: sec(26.55),
    securityLayer: sec(29.45),
    controlPlane: sec(32.55),
    doneBefore: sec(35.65),
    builtIt: sec(37.45),
    blackTwo: sec(38.65),
    close: sec(38.95),
    end: sec(42.6),
  };
  const blackBeat =
    (frame >= t.blackOne && frame < t.infrastructure) ||
    (frame >= t.blackTwo && frame < t.close);

  return (
    <AbsoluteFill style={{ background: black }}>
      {!blackBeat ? (
        <BlackField
          haze={frame < t.everythingOpenSource ? 0.16 : 0.24}
          redHaze={
            frame >= t.openSourceRunsOn && frame < t.blackOne
              ? 0.32
              : frame >= t.securityLayer
                ? 0.18
                : 0
          }
        />
      ) : null}
      {!blackBeat ? (
        <>
          <PulseOverlay start={t.introducing} end={t.everythingOpenSource} strength={0.75} />
          <PulseOverlay start={t.openSourceRunsOn} end={t.blackOne} strength={0.9} />
          <PulseOverlay start={t.securityLayer} end={t.doneBefore} strength={0.72} />
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
            variant="quiet"
            y={-12}
          />
          <KineticLine
            text="So I did it again"
            start={t.didItAgain}
            end={t.introducing}
            size={112}
            variant="impact"
          />
          <KineticLine
            text="Introducing Automic Vault"
            start={t.introducing}
            end={t.everythingOpenSource}
            size={106}
            accent="red"
            variant="product"
          />
          <KineticLine
            text="Everything runs on open source"
            start={t.everythingOpenSource}
            end={t.agentsOpenSource}
            size={76}
            variant="quiet"
          />
          <KineticLine
            text="Agents run on open source"
            start={t.agentsOpenSource}
            end={t.openSourceRunsOn}
            size={104}
            accent="red"
            variant="impact"
          />
          <OpenSourceRunsOn start={t.openSourceRunsOn} end={t.blackOne} />
          <KineticLine
            text="We need better infrastructure."
            start={t.infrastructure}
            end={t.toolsLayer}
            size={88}
            variant="impact"
          />
          <KineticLine
            text="We need it at the layer where tools actually live."
            start={t.toolsLayer}
            end={t.securityLayer}
            size={74}
            maxWidth={1400}
            variant="statement"
          />
          <KineticLine
            text="What if the package manager was also the security layer?"
            start={t.securityLayer}
            end={t.controlPlane}
            size={72}
            accent="red"
            maxWidth={1460}
            variant="question"
          />
          <KineticLine
            text="Also the execution control plane?"
            start={t.controlPlane}
            end={t.doneBefore}
            size={90}
            accent="red"
            variant="impact"
          />
          <KineticLine
            text="I've done this before."
            start={t.doneBefore}
            end={t.builtIt}
            size={94}
            variant="quiet"
          />
          <Sequence from={t.builtIt} durationInFrames={t.blackTwo - t.builtIt}>
            <SoBuiltIt />
          </Sequence>
          <Sequence from={t.close} durationInFrames={t.end - t.close}>
            <Close />
          </Sequence>
        </>
      ) : null}
      {!blackBeat && frame < t.blackTwo ? (
        <AbsoluteFill
          style={{
            opacity: 0.12,
            border: `1px solid ${line}`,
            inset: 46,
            width: "auto",
            height: "auto",
          }}
        />
      ) : null}
    </AbsoluteFill>
  );
};
