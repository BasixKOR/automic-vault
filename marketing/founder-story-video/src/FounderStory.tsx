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
}> = ({ text, start, end, size = 74, weight = 800, muted = false, accent }) => {
  const frame = useCurrentFrame();
  const inAmount = fade(frame, start, start + 10);
  const outAmount = interpolate(frame, [end - 10, end], [1, 0], clamp);
  const lift = interpolate(frame, [start, start + 14], [24, 0], {
    ...clamp,
    easing: softEase,
  });
  const scale = interpolate(frame, [start, start + 14], [0.982, 1], {
    ...clamp,
    easing: softEase,
  });

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
          maxWidth: 1540,
          color: accent === "red" ? red : accent === "green" ? green : muted ? inkMuted : ink,
          fontFamily: sans,
          fontSize: size,
          fontWeight: weight,
          letterSpacing: 0,
          lineHeight: 1.08,
          textAlign: "center",
          transform: `translateY(${lift}px) scale(${scale})`,
          textShadow:
            accent === "red"
              ? "0 0 34px rgba(216,58,47,0.28), 0 26px 48px rgba(0,0,0,0.66)"
              : "0 26px 48px rgba(0,0,0,0.66)",
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
        const scale = visible
          ? interpolate(frame, [item.start, end], [0.986, 1.014], {
              ...clamp,
              easing: softEase,
            })
          : 1;

        return (
          <AbsoluteFill
            key={`${item.word}-${item.start}`}
            style={{
              alignItems: "center",
              justifyContent: "center",
              opacity: visible ? 1 : 0,
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
                    ? "0 0 34px rgba(216,58,47,0.3), 0 28px 50px rgba(0,0,0,0.72)"
                    : "0 28px 50px rgba(0,0,0,0.72)",
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

  const bulletStyle: React.CSSProperties = {
    position: "absolute",
    left: 176,
    top: 328,
    maxWidth: 1260,
    color: red,
    fontFamily: sans,
    fontSize: 92,
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
        }}
      >
        - Plain Text Secrets.
      </div>
      <div
        style={{
          ...bulletStyle,
          opacity: bulletOpacity(secondStart, end),
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
              ? 0.22
              : frame >= t.securityLayer
                ? 0.12
                : 0
          }
        />
      ) : null}
      {!blackBeat ? (
        <>
          <TerminalLine
            text="I created Homebrew"
            start={t.homebrew}
            typeDuration={sec(1.2)}
            y={348}
            holdUntil={t.agentsDawn}
            cursorUntil={sec(1.7)}
          />
          <TerminalLine
            text="At the dawn of Web 2.0"
            start={t.web}
            typeDuration={sec(1.25)}
            y={452}
            holdUntil={t.agentsDawn}
            size={52}
            muted
            cursorUntil={sec(4.1)}
          />
          <KineticLine
            text="It's now the dawn of agents"
            start={t.agentsDawn}
            end={t.didItAgain}
            size={82}
          />
          <KineticLine
            text="So I did it again"
            start={t.didItAgain}
            end={t.introducing}
            size={84}
          />
          <KineticLine
            text="Introducing Automic Vault"
            start={t.introducing}
            end={t.everythingOpenSource}
            size={88}
            accent="red"
          />
          <KineticLine
            text="Everything runs on open source"
            start={t.everythingOpenSource}
            end={t.agentsOpenSource}
            size={78}
          />
          <KineticLine
            text="Agents run on open source"
            start={t.agentsOpenSource}
            end={t.openSourceRunsOn}
            size={82}
            accent="red"
          />
          <OpenSourceRunsOn start={t.openSourceRunsOn} end={t.blackOne} />
          <KineticLine
            text="We need better infrastructure."
            start={t.infrastructure}
            end={t.toolsLayer}
            size={76}
          />
          <KineticLine
            text="We need it at the layer where tools actually live."
            start={t.toolsLayer}
            end={t.securityLayer}
            size={66}
          />
          <KineticLine
            text="What if the package manager was also the security layer?"
            start={t.securityLayer}
            end={t.controlPlane}
            size={64}
            accent="red"
          />
          <KineticLine
            text="Also the execution control plane?"
            start={t.controlPlane}
            end={t.doneBefore}
            size={76}
            accent="red"
          />
          <KineticLine
            text="I've done this before."
            start={t.doneBefore}
            end={t.builtIt}
            size={72}
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
