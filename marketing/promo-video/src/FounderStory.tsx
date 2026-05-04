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
const line = "rgba(230, 215, 178, 0.14)";
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';
const sans =
  '"Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';

const fps = 30;
const sec = (value: number) => Math.round(value * fps);
export const founderStoryDurationInFrames = sec(40);

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

const BlackField: React.FC<{ haze?: number }> = ({ haze = 0.24 }) => (
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
}> = ({ text, start, end, size = 74, weight = 800, muted = false }) => {
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
          color: muted ? inkMuted : ink,
          fontFamily: sans,
          fontSize: size,
          fontWeight: weight,
          letterSpacing: 0,
          lineHeight: 1.08,
          textAlign: "center",
          transform: `translateY(${lift}px) scale(${scale})`,
          textShadow: "0 26px 48px rgba(0,0,0,0.66)",
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
}> = ({ words, start, weights, size = 122 }) => {
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
                color: ink,
                fontFamily: sans,
                fontSize: size,
                fontWeight: 850,
                letterSpacing: 0,
                lineHeight: 1,
                textAlign: "center",
                transform: `scale(${scale})`,
                textShadow: "0 28px 50px rgba(0,0,0,0.72)",
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

const HingeLine: React.FC = () => {
  const frame = useCurrentFrame();
  const start = sec(26.95);
  const end = sec(29.65);
  const opacity = fade(frame, start, start + 16) * interpolate(frame, [end - 14, end], [1, 0], clamp);
  const scale = interpolate(frame, [start, start + 22, end], [0.972, 1, 1.012], {
    ...clamp,
    easing: softEase,
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
          maxWidth: 1460,
          color: ink,
          fontFamily: sans,
          fontSize: 76,
          fontWeight: 700,
          letterSpacing: 0,
          lineHeight: 1.13,
          textAlign: "center",
          transform: `scale(${scale})`,
          textShadow: "0 24px 48px rgba(0,0,0,0.72)",
        }}
      >
        And we just... let agents loose on all of it.
      </div>
    </AbsoluteFill>
  );
};

const Idea: React.FC = () => {
  const frame = useCurrentFrame();
  const firstStart = sec(30.45);
  const secondStart = sec(31.25);
  const close = sec(34);
  const lineStyle: React.CSSProperties = {
    color: ink,
    fontFamily: sans,
    fontSize: 68,
    fontWeight: 800,
    letterSpacing: 0,
    lineHeight: 1.08,
    textAlign: "center",
    textShadow: "0 26px 48px rgba(0,0,0,0.66)",
  };
  const lineOpacity = (start: number, end: number) =>
    fade(frame, start, start + 10) * interpolate(frame, [end - 10, end], [1, 0], clamp);

  return (
    <AbsoluteFill>
      <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
        <div
          style={{
            ...lineStyle,
            opacity: lineOpacity(firstStart, sec(33.1)),
            transform: `translateY(${interpolate(frame, [firstStart, firstStart + 14], [10, -46], {
              ...clamp,
              easing: softEase,
            })}px)`,
          }}
        >
          What if package management, secrets,
        </div>
        <div
          style={{
            ...lineStyle,
            position: "absolute",
            opacity: lineOpacity(secondStart, sec(33.1)),
            transform: `translateY(${interpolate(
              frame,
              [secondStart, secondStart + 14],
              [54, 44],
              {
                ...clamp,
                easing: softEase,
              },
            )}px)`,
          }}
        >
          and execution control were the same thing?
        </div>
      </AbsoluteFill>
      <AbsoluteFill
        style={{
          alignItems: "center",
          justifyContent: "center",
          opacity:
            fade(frame, sec(33.15), sec(33.5)) *
            interpolate(frame, [close - 8, close], [1, 0], clamp),
        }}
      >
        <div
          style={{
            color: ink,
            fontFamily: mono,
            fontSize: 58,
            fontWeight: 700,
            letterSpacing: 0,
            transform: `translateY(${interpolate(frame, [sec(33.15), sec(33.55)], [18, 0], {
              ...clamp,
              easing: softEase,
            })}px)`,
          }}
        >
          Nobody had done it.
        </div>
      </AbsoluteFill>
    </AbsoluteFill>
  );
};

const SoBuiltIt: React.FC = () => {
  return (
    <WordFlashSequence
      words={["so", "i", "built", "it."]}
      start={0}
      weights={[8, 6, 16, 10]}
      size={178}
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
      <AbsoluteFill
        style={{
          opacity: flash * 0.26,
          background:
            "radial-gradient(circle at center, rgba(124,255,188,0.34), transparent 21%)",
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
          filter: "drop-shadow(0 0 30px rgba(124,255,188,0.24))",
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
  const realizationLines = [
    ["Open source ecosystems.", sec(18.95), sec(20.75), 84],
    ["Full of plain text secrets.", sec(21.0), sec(22.75), 82],
    ["Commands that can delete prod.", sec(23.0), sec(24.9), 82],
    ["With one line.", sec(25.15), sec(26.35), 94],
  ] as const;
  const blackBeat =
    (frame >= sec(26.55) && frame < sec(26.6)) ||
    (frame >= sec(29.65) && frame < sec(30.45)) ||
    (frame >= sec(34) && frame < sec(34.18));

  return (
    <AbsoluteFill style={{ background: black }}>
      {!blackBeat ? <BlackField haze={frame < sec(16) ? 0.16 : 0.24} /> : null}
      {!blackBeat ? (
        <>
          <TerminalLine
            text="I made Homebrew."
            start={sec(0.8)}
            typeDuration={sec(1.15)}
            y={390}
            holdUntil={sec(7.85)}
            cursorUntil={sec(3.1)}
          />
          <TerminalLine
            text="That was 17 years ago."
            start={sec(4.05)}
            typeDuration={sec(1.35)}
            y={492}
            holdUntil={sec(7.85)}
            muted
            cursorUntil={sec(7)}
          />
          <KineticLine
            text="Been fairly quiet since."
            start={sec(8.35)}
            end={sec(11.7)}
            size={70}
            muted
          />
          <KineticLine text="Then agents happened." start={sec(12.2)} end={sec(15.8)} size={86} />
          <WordFlashSequence
            words={["I", "started", "looking", "at", "what", "agents", "actually", "run", "on."]}
            start={sec(16.15)}
            weights={[5, 10, 9, 5, 6, 8, 10, 5, 6]}
          />
          {realizationLines.map(([text, start, end, size]) => (
            <KineticLine key={text} text={text} start={start} end={end} size={size} />
          ))}
          <HingeLine />
          <Idea />
          <Sequence from={sec(34.18)} durationInFrames={sec(1.52)}>
            <SoBuiltIt />
          </Sequence>
          <Sequence from={sec(35.9)} durationInFrames={sec(4.1)}>
            <Close />
          </Sequence>
        </>
      ) : null}
      {!blackBeat && frame < sec(34) ? (
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
