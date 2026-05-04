import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  interpolateColors,
  spring,
  staticFile,
  useCurrentFrame,
  useVideoConfig,
} from "remotion";

const red = "#d83a2f";
const green = "#6bffb0";
const amber = "#ffb347";
const black = "#0a0d10";
const deepBlack = "#030506";
const ink = "#d6c7a1";
const inkMuted = "#b89b73";
const line = "rgba(214, 199, 161, 0.45)";
const lineFaint = "rgba(214, 199, 161, 0.2)";
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';
const sans =
  '"Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const display =
  '"Barlow Condensed", "Arial Narrow", Impact, ui-sans-serif, system-ui, sans-serif';

const sec = (value: number) => Math.round(value * 30);
const storyDuration = sec(6.55);
const headlineStart = storyDuration;
const headlineDuration = sec(3);
const logoDuration = sec(2.8);
const tokenUseCaseDuration = sec(16.9);
const standardUseCaseDuration = sec(6);
const statementScreenDuration = sec(5.6);
const logoStart = headlineStart + headlineDuration;
const useCasesStart = logoStart + logoDuration;
const publishStart = useCasesStart + tokenUseCaseDuration;
const publishStatementStart = publishStart + standardUseCaseDuration;
const installStart = publishStatementStart + statementScreenDuration;
const closeStart = installStart + standardUseCaseDuration;
export const durationInFrames = closeStart + sec(2);

const cutOpacity = (frame: number, start: number, end: number) =>
  frame >= start && frame < end ? 1 : 0;

const SectionBackground: React.FC = () => (
  <AbsoluteFill style={{ background: deepBlack }}>
    <AbsoluteFill
      style={{
        background:
          "radial-gradient(circle at 74% 50%, rgba(216, 58, 47, 0.1), transparent 24%), linear-gradient(180deg, #030506 0%, #0a0d10 54%, #030506 100%)",
      }}
    />
    <AbsoluteFill
      style={{
        opacity: 0.085,
        mixBlendMode: "screen",
        background:
          "radial-gradient(circle at 23% 12%, rgba(255, 179, 71, 0.06), transparent 9%), repeating-linear-gradient(180deg, rgba(255, 255, 255, 0.05) 0, rgba(255, 255, 255, 0.05) 1px, transparent 1px, transparent 4px)",
      }}
    />
    <AbsoluteFill
      style={{
        background:
          "linear-gradient(180deg, rgba(58, 74, 82, 0.12), transparent 36%), rgba(10, 13, 16, 0.9)",
      }}
    />
    <AbsoluteFill
      style={{
        background:
          "radial-gradient(circle at center, transparent 42%, rgba(0, 0, 0, 0.56)), linear-gradient(90deg, transparent, rgba(216, 58, 47, 0.04), transparent), repeating-linear-gradient(90deg, rgba(214, 199, 161, 0.012) 0, rgba(214, 199, 161, 0.012) 1px, transparent 1px, transparent 64px)",
      }}
    />
    <AbsoluteFill
      style={{
        borderTop: `1px solid ${lineFaint}`,
        borderBottom: `1px solid ${lineFaint}`,
      }}
    />
  </AbsoluteFill>
);

const SiteBackground: React.FC<{ gridOpacity?: number; vignetteOpacity?: number }> = ({
  gridOpacity = 0.48,
  vignetteOpacity = 0.56,
}) => (
  <AbsoluteFill style={{ background: deepBlack }}>
    <RedGrid opacity={gridOpacity} vignetteOpacity={vignetteOpacity} />
    <AbsoluteFill
      style={{
        opacity: 0.2,
        background:
          "repeating-linear-gradient(0deg, rgba(214,199,161,0.026) 0, rgba(214,199,161,0.026) 1px, transparent 1px, transparent 7px)",
      }}
    />
    <AbsoluteFill
      style={{
        background:
          "radial-gradient(circle at center, transparent 42%, rgba(0,0,0,0.56)), linear-gradient(90deg, transparent, rgba(216,58,47,0.04), transparent), repeating-linear-gradient(90deg, rgba(214,199,161,0.012) 0, rgba(214,199,161,0.012) 1px, transparent 1px, transparent 64px)",
      }}
    />
  </AbsoluteFill>
);

const TerminalCursor: React.FC<{
  color: string;
  dangerLevel: number;
  desaturationLevel: number;
  empty: boolean;
}> = ({ color, dangerLevel, desaturationLevel, empty }) => {
  const frame = useCurrentFrame();
  const glow = interpolate(desaturationLevel, [0, 1], [0.62, 0]);
  const scan = interpolate(frame % 18, [0, 9, 18], [0, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const shadowColor =
    dangerLevel > 0.5 ? "rgba(216, 58, 47, 0.58)" : "rgba(107, 255, 176, 0.48)";

  return (
    <span
      style={{
        position: "relative",
        display: "inline-block",
        flex: "0 0 auto",
        width: 9,
        height: 58,
        marginLeft: empty ? 0 : 12,
        borderRadius: 2,
        background: color,
        boxShadow: glow > 0 ? `0 0 20px ${shadowColor}` : "none",
        overflow: "hidden",
      }}
    >
      <span
        style={{
          position: "absolute",
          inset: "0 0 auto 0",
          height: 18,
          opacity: (1 - desaturationLevel) * scan * 0.32,
          background: "rgba(255, 255, 255, 0.82)",
          transform: `translateY(${scan * 40}px)`,
        }}
      />
    </span>
  );
};

const TerminalLine: React.FC<{
  text: string;
  y: number;
  visible: boolean;
  danger?: boolean;
  dangerAmount?: number;
  desaturated?: boolean;
  desaturationAmount?: number;
  opacityMultiplier?: number;
  cursor?: boolean;
  glitchAmount?: number;
}> = ({
  text,
  y,
  visible,
  danger = false,
  dangerAmount,
  desaturated = false,
  desaturationAmount,
  opacityMultiplier = 1,
  cursor = false,
  glitchAmount = 0,
}) => {
  const frame = useCurrentFrame();
  const dangerLevel = danger ? (dangerAmount ?? 1) : 0;
  const activeColor = interpolateColors(dangerLevel, [0, 1], [green, red]);
  const desaturationLevel = desaturationAmount ?? (desaturated ? 1 : 0);
  const lineColor = interpolateColors(desaturationLevel, [0, 1], [activeColor, inkMuted]);
  const lineOpacity = visible
    ? interpolate(desaturationLevel, [0, 1], [1, 0.54]) * opacityMultiplier
    : 0;
  const cursorColor = interpolateColors(desaturationLevel, [0, 1], [activeColor, inkMuted]);
  const glitchX = ((frame * 23) % 17) - 8;

  return (
    <div
      style={{
        position: "absolute",
        left: 138,
        top: y,
        opacity: lineOpacity,
        color: lineColor,
        fontFamily: mono,
        fontSize: 54,
        fontWeight: 800,
        height: 76,
        letterSpacing: 0,
        lineHeight: "76px",
        display: "flex",
        alignItems: "center",
        filter: desaturationLevel > 0 ? `grayscale(${desaturationLevel})` : undefined,
        textShadow: desaturationLevel >= 1
          ? "none"
          : dangerLevel > 0.5
            ? "0 0 18px rgba(216, 58, 47, 0.42)"
          : "0 0 16px rgba(107, 255, 176, 0.24)",
      }}
    >
      <span style={{ position: "relative", display: "inline-block" }}>
        {text}
        {glitchAmount > 0 && desaturationLevel < 1
          ? [
              { color: red, y: -9, x: glitchX },
              { color: ink, y: 0, x: -glitchX * 0.7 },
              { color: red, y: 9, x: glitchX * 0.45 },
            ].map((layer, index) => (
              <span
                key={`${layer.color}-${layer.y}`}
                style={{
                  position: "absolute",
                  left: 0,
                  top: 0,
                  color: layer.color,
                  opacity: glitchAmount * (1 - desaturationLevel) * (0.55 - index * 0.1),
                  transform: `translate(${layer.x}px, ${layer.y}px)`,
                  clipPath: `inset(${index * 29}% 0 ${58 - index * 18}% 0)`,
                  textShadow:
                    layer.color === red
                      ? "0 0 20px rgba(216, 58, 47, 0.72)"
                      : "0 0 18px rgba(107, 255, 176, 0.42)",
                }}
              >
                {text}
              </span>
            ))
          : null}
      </span>
      {cursor ? (
        <TerminalCursor
          color={cursorColor}
          dangerLevel={dangerLevel}
          desaturationLevel={desaturationLevel}
          empty={text.length === 0}
        />
      ) : null}
    </div>
  );
};

const typedText = (text: string, frame: number, start: number, frames: number) => {
  if (frame < start) {
    return "";
  }

  const progress = Math.min(1, (frame - start + 1) / frames);

  return text.slice(0, Math.ceil(text.length * progress));
};

const Story: React.FC = () => {
  const frame = useCurrentFrame();
  const lineOne = "/// OUR AGENTS WERE AUTONOMOUS.";
  const lineTwo = "/// THEY DELETED PROD.";
  const lineThree = "/// NEVER AGAIN.";
  const lineOnePending = 0;
  const lineOneStart = 24;
  const lineOneTyped = 14;
  const lineTwoPending = 55;
  const lineTwoStart = 70;
  const lineTwoTyped = 11;
  const lineTwoComplete = lineTwoStart + lineTwoTyped;
  const lineTwoRedStart = lineTwoComplete + 28;
  const lineThreePending = 136;
  const lineThreeStart = 154;
  const lineThreeTyped = 9;
  const lineThreeComplete = lineThreeStart + lineThreeTyped;
  const lineThreeBlinkStart = lineThreeComplete + 8;
  const glitchStart = sec(6.05);
  const lineTwoDanger = interpolate(
    frame,
    [lineTwoRedStart, lineTwoRedStart + 8],
    [0, 1],
    {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
      easing: Easing.bezier(0.16, 1, 0.3, 1),
    },
  );
  const lineTwoGlitch = interpolate(
    frame,
    [lineTwoRedStart - 2, lineTwoRedStart + 4, lineTwoRedStart + 12],
    [0, 1, 0],
    {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
      easing: Easing.bezier(0.16, 1, 0.3, 1),
    },
  );
  const glitch = interpolate(frame, [glitchStart, storyDuration], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.7, 0, 0.84, 0),
  });
  const lineOneDesaturation = interpolate(frame, [lineTwoStart, lineTwoStart + 12], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.16, 1, 0.3, 1),
  });
  const lineTwoDesaturation = interpolate(frame, [lineThreeStart, lineThreeStart + 12], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.16, 1, 0.3, 1),
  });
  const lineThreeBlinkPhase = Math.floor((frame - lineThreeBlinkStart) / 3);
  const lineThreeOpacity =
    frame >= lineThreeBlinkStart && frame < glitchStart + 8
      ? lineThreeBlinkPhase % 2 === 0
        ? 0.1
        : 1
      : 1;
  const jitter = frame >= glitchStart ? ((frame * 37) % 29) - 14 : 0;
  const contentStyle = {
    opacity: interpolate(glitch, [0, 0.72, 1], [1, 1, 0]),
    filter: `blur(${glitch * 18}px) contrast(${1 + glitch * 2.4})`,
    transform: `translateX(${jitter * glitch * 1.6}px) scale(${1 + glitch * 0.05})`,
  };

  return (
    <AbsoluteFill style={{ background: deepBlack }}>
      <SectionBackground />
      <AbsoluteFill style={contentStyle}>
        <TerminalLine
          text={typedText(lineOne, frame, lineOneStart, lineOneTyped)}
          y={330}
          visible={frame >= lineOnePending && frame < storyDuration}
          desaturationAmount={lineOneDesaturation}
          cursor={frame >= lineOnePending && frame < lineTwoPending}
        />
        <TerminalLine
          text={typedText(lineTwo, frame, lineTwoStart, lineTwoTyped)}
          y={440}
          visible={frame >= lineTwoPending && frame < storyDuration}
          danger
          dangerAmount={lineTwoDanger}
          desaturationAmount={lineTwoDesaturation}
          cursor={frame >= lineTwoPending && frame < lineThreePending}
          glitchAmount={lineTwoGlitch}
        />
        <TerminalLine
          text={typedText(lineThree, frame, lineThreeStart, lineThreeTyped)}
          y={550}
          visible={frame >= lineThreePending && frame < storyDuration}
          opacityMultiplier={lineThreeOpacity}
          cursor={frame >= lineThreePending && frame < glitchStart}
        />
      </AbsoluteFill>
      {frame >= glitchStart ? (
        <>
          {[0, 1, 2, 3].map((band) => (
            <div
              key={band}
              style={{
                position: "absolute",
                left: 0,
                top: 238 + band * 122 + ((frame * (band + 3)) % 36),
                width: "100%",
                height: 24 + band * 9,
                opacity: glitch * (0.46 + band * 0.1),
                background:
                  band % 2 === 0
                    ? "rgba(216, 58, 47, 0.78)"
                    : "rgba(255, 179, 71, 0.38)",
                filter: `blur(${2 + glitch * 12}px)`,
                transform: `translateX(${(((frame + band) * 19) % 95 - 48) * glitch}px)`,
              }}
            />
          ))}
          <AbsoluteFill
            style={{
              opacity: glitch * 0.18,
              background:
                "repeating-linear-gradient(0deg, transparent 0, transparent 7px, rgba(255,255,255,0.9) 8px)",
              mixBlendMode: "screen",
            }}
          />
        </>
      ) : null}
    </AbsoluteFill>
  );
};

const CrashWord: React.FC<{ word: string; start: number; end: number }> = ({
  word,
  start,
  end,
}) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const local = frame - start;
  const amount = spring({
    frame: local,
    fps,
    config: { damping: 14, stiffness: 280, mass: 0.58 },
  });
  const scale = interpolate(amount, [0, 1], [1.55, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "extend",
  });
  const rotate = interpolate(amount, [0, 1], [-2.5, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "extend",
  });

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: cutOpacity(frame, start, end),
        background: "transparent",
      }}
    >
      <SectionBackground />
      <div
        style={{
          color: ink,
          fontFamily: display,
          fontSize: word === "APPROVAL" ? 216 : 260,
          fontWeight: 800,
          letterSpacing: "0.035em",
          lineHeight: 0.88,
          transform: `scale(${scale}) rotate(${rotate}deg)`,
          textShadow: "0 18px 28px rgba(0, 0, 0, 0.62)",
        }}
      >
        {word}
      </div>
    </AbsoluteFill>
  );
};

const RedGrid: React.FC<{ opacity?: number; vignetteOpacity?: number }> = ({
  opacity = 1,
  vignetteOpacity = 0.92,
}) => {
  return (
    <AbsoluteFill style={{ opacity }}>
      <Img
        src={staticFile("rocket-grid.png")}
        style={{
          width: "100%",
          height: "100%",
          objectFit: "cover",
          filter: "contrast(1.08) saturate(0.78) sepia(0.18) brightness(0.62)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            `linear-gradient(90deg, rgba(10,13,16,0.9) 0%, rgba(10,13,16,0.38) 48%, rgba(3,5,6,0.86) 100%), radial-gradient(circle at 58% 78%, rgba(216,58,47,0.28), transparent 16%), radial-gradient(circle at 50% 44%, transparent 0, rgba(3,5,6,0.04) 36%, rgba(3,5,6,${vignetteOpacity}) 82%)`,
        }}
      />
    </AbsoluteFill>
  );
};

const Headline: React.FC = () => {
  return (
    <AbsoluteFill style={{ background: deepBlack }}>
      <SectionBackground />
      <CrashWord word="HUMAN" start={sec(0)} end={sec(1)} />
      <CrashWord word="APPROVAL" start={sec(1)} end={sec(2)} />
      <CrashWord word="REQUIRED." start={sec(2)} end={sec(3)} />
    </AbsoluteFill>
  );
};

const RequiredFadeOut: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 6, 22], [1, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.18, 0.82, 0.28, 1),
  });
  const blur = interpolate(frame, [4, 22], [0, 10], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const scale = interpolate(frame, [0, 22], [1, 0.985], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity,
        background: "transparent",
        zIndex: 25,
      }}
    >
      <div
        style={{
          color: ink,
          fontFamily: display,
          fontSize: 260,
          fontWeight: 800,
          letterSpacing: "0.035em",
          lineHeight: 0.88,
          filter: `blur(${blur}px)`,
          transform: `scale(${scale})`,
          textShadow: "0 18px 28px rgba(0, 0, 0, 0.62)",
        }}
      >
        REQUIRED.
      </div>
    </AbsoluteFill>
  );
};

const HeadlineToLogoTransition: React.FC = () => {
  const frame = useCurrentFrame();
  const opacity = interpolate(frame, [0, 3, 13, 17], [0, 1, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const crush = interpolate(frame, [0, 8, 17], [0, 1, 0], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.76, 0, 0.24, 1),
  });

  return (
    <AbsoluteFill style={{ opacity, zIndex: 30, background: "transparent" }}>
      <AbsoluteFill
        style={{
          opacity: 0.82 * crush,
          background:
            "radial-gradient(circle at center, rgba(216,58,47,0.22), transparent 28%), rgba(3,5,6,0.78)",
          filter: `contrast(${1 + crush * 1.8}) blur(${crush * 4}px)`,
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18 + crush * 0.3,
          background:
            "repeating-linear-gradient(0deg, transparent 0, transparent 8px, rgba(255,255,255,0.82) 9px), repeating-linear-gradient(90deg, transparent 0, transparent 17px, rgba(216,58,47,0.5) 18px)",
          mixBlendMode: "screen",
          transform: `translateX(${(((frame * 41) % 33) - 16) * crush}px)`,
        }}
      />
    </AbsoluteFill>
  );
};

const LogoHold: React.FC<{ close?: boolean }> = ({ close = false }) => {
  const frame = useCurrentFrame();
  const logoOpacity = close
    ? interpolate(frame, [0, 18], [0, 1], { extrapolateRight: "clamp" })
    : 1;
  const wordOpacity = interpolate(frame, close ? [12, 30] : [30, 54], [0, 1], {
    extrapolateRight: "clamp",
  });
  const stampStart = 34;
  const stampOpacity = close
    ? interpolate(frame, [stampStart - 1, stampStart], [0, 1], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 0;
  const stampScale = interpolate(frame, [stampStart, stampStart + 6], [2.35, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.78, 0, 1, 1),
  });
  const stampInk = interpolate(frame, [stampStart, stampStart + 8, stampStart + 18], [0, 1, 0.86], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });
  const wordmarkWidth = 728;
  const wordmarkHeight = 338;

  return (
    <AbsoluteFill
      style={{
        background: deepBlack,
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <SiteBackground gridOpacity={0.62} vignetteOpacity={0.58} />
      <Img
        src={staticFile("icon.png")}
        style={{
          width: 300,
          height: 300,
          objectFit: "contain",
          opacity: logoOpacity,
          transform: `translateY(${close ? -70 : -36}px)`,
          filter: "drop-shadow(0 0 34px rgba(216, 58, 47, 0.34))",
        }}
      />
      <Img
        src={staticFile("wordmark.png")}
        style={{
          position: "absolute",
          top: close ? 552 : 588,
          width: wordmarkWidth,
          height: wordmarkHeight,
          objectFit: "contain",
          opacity: wordOpacity,
          filter: "drop-shadow(0 18px 22px rgba(0, 0, 0, 0.58))",
        }}
      />
      {close ? (
        <div
          style={{
            position: "absolute",
            left: 600,
            top: 772,
            width: 720,
            height: 182,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            border: `6px solid ${red}`,
            borderRadius: 22,
            background: "rgba(2, 4, 5, 0.88)",
            color: red,
            fontFamily: display,
            fontSize: 78,
            fontWeight: 800,
            letterSpacing: "0.035em",
            lineHeight: 0.86,
            textAlign: "center",
            textTransform: "uppercase",
            opacity: stampOpacity,
            transform: `rotate(-7deg) scale(${stampScale})`,
            boxShadow:
              "inset 0 0 0 3px rgba(216, 58, 47, 0.5), 0 22px 48px rgba(0,0,0,0.5)",
            filter: `contrast(${1 + stampInk * 0.28}) saturate(${0.88 + stampInk * 0.26})`,
            zIndex: 20,
          }}
        >
          <div
            style={{
              position: "absolute",
              inset: 10,
              border: `2px solid ${red}`,
              borderRadius: 14,
              opacity: 0.72,
            }}
          />
          <div
            style={{
              position: "absolute",
              inset: 0,
              opacity: 0.18 * stampInk,
              background:
                "repeating-linear-gradient(0deg, transparent 0, transparent 5px, rgba(216,58,47,0.85) 6px), repeating-linear-gradient(90deg, transparent 0, transparent 11px, rgba(216,58,47,0.42) 12px)",
              mixBlendMode: "screen",
            }}
          />
          <span
            style={{
              position: "relative",
              display: "block",
              textShadow: "0 3px 0 rgba(0, 0, 0, 0.5)",
            }}
          >
            Human Approval Required
          </span>
        </div>
      ) : null}
    </AbsoluteFill>
  );
};

const TerminalPanel: React.FC<{
  command: string;
  response?: string;
  typedFrames: number;
  frozen?: boolean;
  prompt?: string;
  responseColor?: string;
}> = ({ command, response, typedFrames, frozen = false, prompt = "$ ", responseColor }) => {
  const frame = useCurrentFrame();
  const chars = Math.min(command.length, Math.floor(frame / typedFrames));
  const commandText = command.slice(0, chars);

  return (
    <div
      style={{
        position: "absolute",
        left: 115,
        top: 160,
        width: 1010,
        height: 660,
        borderRadius: 8,
        background:
          "linear-gradient(180deg, rgba(23,33,38,0.54), rgba(0,0,0,0.28)), rgba(0,0,0,0.34)",
        border: `1px solid ${lineFaint}`,
        boxShadow:
          "inset 0 1px 0 rgba(214,199,161,0.08), 0 28px 80px -58px rgba(255,179,71,0.32)",
        overflow: "hidden",
        filter: frozen ? "grayscale(0.7) brightness(0.65)" : undefined,
      }}
    >
      <div
        style={{
          height: 46,
          background: "rgba(5, 8, 9, 0.92)",
          borderBottom: `1px solid ${lineFaint}`,
        }}
      />
      <div
        style={{
          padding: 46,
          color: green,
          fontFamily: mono,
          fontSize: 42,
          lineHeight: 1.45,
        }}
      >
        {prompt ? <span style={{ color: amber }}>{prompt}</span> : null}
        {commandText}
        <span style={{ opacity: frame % 18 < 9 ? 1 : 0 }}>_</span>
        {response ? (
          <div
            style={{
              marginTop: 38,
              color: responseColor ?? (response.includes("required") ? red : green),
              fontWeight: response === "HUMAN APPROVAL REQUIRED" ? 900 : undefined,
              textTransform: response === "HUMAN APPROVAL REQUIRED" ? "uppercase" : undefined,
            }}
          >
            {response}
          </div>
        ) : null}
      </div>
    </div>
  );
};

const ScreenLabel: React.FC<{ text: string }> = ({ text }) => {
  return (
    <div
      style={{
        position: "absolute",
        left: 72,
        top: 56,
        color: inkMuted,
        fontFamily: mono,
        fontSize: 28,
        fontWeight: 700,
        letterSpacing: "0.08em",
        textTransform: "uppercase",
      }}
    >
      {text}
    </div>
  );
};

const MouseCursor: React.FC<{
  x: number;
  y: number;
  pressed: boolean;
}> = ({ x, y, pressed }) => {
  const scale = pressed ? 0.9 : 1;

  return (
    <svg
      viewBox="0 0 48 64"
      style={{
        position: "absolute",
        left: x,
        top: y,
        width: 54,
        height: 72,
        overflow: "visible",
        transform: `rotate(-12deg) scale(${scale})`,
        transformOrigin: "9px 9px",
        filter: "drop-shadow(0 8px 14px rgba(0, 0, 0, 0.42))",
      }}
    >
      <path
        d="M5 4L38 36L23 39L16 58L5 4Z"
        fill="#050505"
        stroke="#fff"
        strokeLinejoin="round"
        strokeWidth="3.2"
      />
    </svg>
  );
};

const ApprovalModal: React.FC<{
  title: string;
  note?: string;
  decision: "DENY" | "APPROVE";
  clickFrame: number;
  clicked: boolean;
}> = ({ title, note, decision, clickFrame, clicked }) => {
  const frame = useCurrentFrame();
  const positive = decision === "APPROVE";
  const cursorProgress = interpolate(frame, [clickFrame - 18, clickFrame - 4], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
    easing: Easing.bezier(0.16, 1, 0.3, 1),
  });
  const cursorStartX = positive ? 472 : 292;
  const cursorTargetX = positive ? 558 : 350;
  const cursorStartY = 238;
  const cursorTargetY = positive ? 218 : 190;
  const cursorX = interpolate(cursorProgress, [0, 1], [cursorStartX, cursorTargetX]);
  const cursorY = interpolate(cursorProgress, [0, 1], [cursorStartY, cursorTargetY]);

  return (
    <div
      style={{
        position: "absolute",
        left: 1040,
        top: 310,
        width: 650,
        minHeight: 330,
        borderRadius: 8,
        background:
          "linear-gradient(180deg, rgba(23,33,38,0.9), rgba(0,0,0,0.62)), rgba(10,13,16,0.95)",
        border: `1px solid ${line}`,
        boxShadow:
          "inset 0 1px 0 rgba(214,199,161,0.08), 0 28px 80px -58px rgba(255,179,71,0.32)",
        color: ink,
        fontFamily: sans,
        padding: "40px 42px",
      }}
    >
      <Img
        src={staticFile("icon.png")}
        style={{
          position: "absolute",
          right: 24,
          top: 20,
          width: 58,
          height: 58,
          objectFit: "contain",
          opacity: 0.16,
          filter: "sepia(0.4) saturate(0.65) contrast(1.2)",
        }}
      />
      <div
        style={{
          position: "absolute",
          inset: 14,
          border: `1px solid ${lineFaint}`,
          pointerEvents: "none",
        }}
      />
      <div
        style={{
          display: "flex",
          gap: 10,
          position: "absolute",
          top: 18,
          left: 24,
        }}
      >
        {[red, amber, green].map((color) => (
          <div
            key={color}
            style={{ width: 13, height: 13, borderRadius: 20, background: color }}
          />
        ))}
      </div>
      <div
        style={{
          fontFamily: display,
          fontSize: 58,
          fontWeight: 800,
          lineHeight: 0.9,
          letterSpacing: "0.035em",
          marginTop: 30,
          textTransform: "uppercase",
        }}
      >
        {title}
      </div>
      {note ? (
        <div style={{ color: inkMuted, fontFamily: mono, fontSize: 24, marginTop: 20 }}>
          {note}
        </div>
      ) : null}
      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          gap: 18,
          marginTop: 34,
        }}
      >
        <button
          style={{
            width: 150,
            height: 58,
            borderRadius: 0,
            border: `1px solid ${lineFaint}`,
            background: decision === "DENY" && clicked ? red : "rgba(10,13,16,0.42)",
            color: decision === "DENY" && clicked ? black : amber,
            fontSize: 24,
            fontFamily: mono,
            fontWeight: 700,
            textTransform: "uppercase",
          }}
        >
          DENY
        </button>
        <button
          style={{
            width: 170,
            height: 58,
            borderRadius: 0,
            border: `1px solid ${positive && clicked ? green : lineFaint}`,
            background: positive && clicked ? green : "rgba(10,13,16,0.42)",
            color: positive && clicked ? black : amber,
            fontSize: 24,
            fontFamily: mono,
            fontWeight: 700,
            textTransform: "uppercase",
          }}
        >
          APPROVE
        </button>
      </div>
      <MouseCursor x={cursorX} y={cursorY} pressed={clicked} />
    </div>
  );
};

const PublishCommandStream: React.FC<{
  pauseFrame: number;
  approvalFrame: number;
  modalVisible: boolean;
}> = ({ pauseFrame, approvalFrame, modalVisible }) => {
  const frame = useCurrentFrame();
  const motionFrame = Math.min(frame, pauseFrame);
  const commands = [
    "git status --short",
    "git rev-parse --show-toplevel",
    "node --version",
    "corepack enable",
    "pnpm --version",
    "jq .version package.json",
    "git diff -- package.json",
    "find src -name '*.ts' -maxdepth 4",
    "rg \"TODO|FIXME\" src",
    "npm run clean",
    "rm -rf dist coverage .turbo",
    "mkdir -p dist",
    "npm run lint",
    "eslint src --max-warnings=0",
    "npm run typecheck",
    "tsc --noEmit",
    "npm run test -- --runInBand",
    "node --test test/*.test.js",
    "vitest run --reporter=dot",
    "npm run build",
    "vite build --mode production",
    "rollup -c",
    "npm run build:css",
    "postcss src/index.css -o dist/index.css",
    "npm run build:server",
    "node scripts/generate-manifest.js",
    "npm run bundle",
    "tar -tf dist/package.tgz",
    "npm run docs",
    "markdownlint README.md docs",
    "npm run test:e2e",
    "playwright test --project=chromium",
    "npm run verify",
    "node scripts/verify-assets.js",
    "sha256sum dist/*",
    "npm run pack",
    "npm pack --dry-run",
    "npm run prepublishOnly",
    "node scripts/update-version.js",
    "node scripts/check-release.js",
    "npm run changelog",
    "git log --oneline -8",
    "npm run audit -- --production",
    "npm audit signatures",
    "npm run size-limit",
    "du -sh dist",
    "npm run smoke",
    "git diff --check",
    "git diff --stat",
    "git ls-files --others --exclude-standard",
    "npm run release:dry",
    "node scripts/preflight-release.js",
    "curl -fsS https://registry.npmjs.org/-/ping",
    "openssl dgst -sha256 dist/*.tgz",
    "git tag --points-at HEAD",
    "npm run clean",
    "npm ci --ignore-scripts",
    "npm run build",
    "npm run test",
    "npm publish",
  ];
  const lineHeight = 42;
  const visibleRows = 10;
  const streamIndex = Math.min(
    commands.length - 1,
    Math.floor(interpolate(motionFrame, [0, pauseFrame], [0, commands.length - 1], {
      extrapolateLeft: "clamp",
      extrapolateRight: "clamp",
    })),
  );
  const firstVisible = Math.max(0, streamIndex - visibleRows + 1);
  const visibleCommands = commands.slice(firstVisible, streamIndex + 1);
  const scrollOffset = frame < pauseFrame ? (frame * 3.8) % lineHeight : 0;
  const approvalOpacity = interpolate(frame, [approvalFrame, approvalFrame + 6], [0, 1], {
    extrapolateLeft: "clamp",
    extrapolateRight: "clamp",
  });

  return (
    <div
      style={{
        position: "absolute",
        left: 154,
        top: 132,
        width: 1180,
        height: 660,
        border: `1px solid ${lineFaint}`,
        borderRadius: 8,
        background: "rgba(3, 5, 6, 0.84)",
        boxShadow: "0 30px 70px rgba(0, 0, 0, 0.5)",
        overflow: "hidden",
        opacity: modalVisible ? 0.46 : 1,
        filter: modalVisible ? "grayscale(0.78)" : undefined,
      }}
    >
      <div
        style={{
          height: 42,
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "0 18px",
          borderBottom: `1px solid ${lineFaint}`,
          background: "rgba(10, 13, 16, 0.76)",
        }}
      >
        {[red, amber, green].map((color) => (
          <div
            key={color}
            style={{
              width: 11,
              height: 11,
              borderRadius: 11,
              background: color,
              opacity: 0.82,
            }}
          />
        ))}
      </div>
      <div
        style={{
          position: "absolute",
          left: 38,
          right: 38,
          top: 76,
          bottom: 40,
          color: ink,
          fontFamily: mono,
          fontSize: 27,
          lineHeight: `${lineHeight}px`,
          fontWeight: 700,
        }}
      >
        <div
          style={{
            transform: `translateY(${-scrollOffset}px)`,
          }}
        >
          {visibleCommands.map((command, index) => {
            const isPublish = command === "npm publish";
            return (
              <div
                key={`${command}-${firstVisible + index}`}
                style={{
                  color: isPublish ? ink : index < visibleCommands.length - 1 ? line : inkMuted,
                  opacity: isPublish ? 1 : 0.78,
                  textShadow: isPublish ? "0 0 22px rgba(214, 199, 161, 0.26)" : undefined,
                }}
              >
                <span style={{ color: amber }}>$ </span>
                {command}
              </div>
            );
          })}
        </div>
        {frame >= approvalFrame ? (
          <div
            style={{
              marginTop: 20,
              color: red,
              fontSize: 36,
              fontWeight: 900,
              letterSpacing: "0.04em",
              opacity: approvalOpacity,
              textTransform: "uppercase",
              textShadow: "0 0 30px rgba(216, 58, 47, 0.42)",
            }}
          >
            HUMAN APPROVAL REQUIRED
          </div>
        ) : null}
      </div>
      <AbsoluteFill
        style={{
          pointerEvents: "none",
          opacity: 0.18,
          background:
            "repeating-linear-gradient(0deg, transparent 0, transparent 6px, rgba(214,199,161,0.28) 7px)",
          mixBlendMode: "screen",
        }}
      />
    </div>
  );
};

const UseCase: React.FC<{
  kind: "token" | "publish" | "install";
}> = ({ kind }) => {
  const frame = useCurrentFrame();
  const clicked = frame >= sec(kind === "token" ? 4.7 : kind === "publish" ? 5.05 : 4);
  const modalStart = sec(kind === "token" ? 2.2 : kind === "publish" ? 3.15 : 1.4);
  const modalVisible = frame >= modalStart;
  const publishPauseFrame = sec(2.25);
  const publishApprovalFrame = sec(2.45);

  return (
    <AbsoluteFill style={{ background: deepBlack }}>
      <SectionBackground />
      {kind === "publish" ? (
        <PublishCommandStream
          pauseFrame={publishPauseFrame}
          approvalFrame={publishApprovalFrame}
          modalVisible={modalVisible}
        />
      ) : (
        <TerminalPanel
          command={kind === "token" ? "gh auth token" : "npm add dodgy-package"}
          response={
            clicked
              ? kind === "install"
                ? "added dodgy-package"
                : "approval required"
              : kind === "token" && frame > sec(1.65)
                ? "gho_********************************"
                : undefined
          }
          typedFrames={kind === "install" ? 2.8 : 2.5}
          frozen={modalVisible && !clicked}
        />
      )}
      {modalVisible ? (
        <ApprovalModal
          title={
            kind === "token"
              ? "Agent wants secret"
              : kind === "publish"
                ? "Agent wants to: npm publish"
                : "Agent Request"
          }
          note={
            kind === "install"
              ? "npm description: A very very very safe and legitimate pkg."
              : undefined
          }
          decision={kind === "install" ? "APPROVE" : "DENY"}
          clickFrame={sec(kind === "token" ? 4.7 : kind === "publish" ? 5.05 : 4)}
          clicked={clicked}
        />
      ) : null}
      {kind === "install" && clicked ? (
        <div
          style={{
            position: "absolute",
            right: 172,
            bottom: 155,
            display: "flex",
            alignItems: "center",
            gap: 18,
            color: green,
            fontFamily: mono,
            fontSize: 32,
          }}
        >
          <div
            style={{
              width: 22,
              height: 22,
              borderRadius: 22,
              background: green,
              boxShadow: "0 0 30px rgba(107, 255, 176, 0.72)",
            }}
          />
          APPROVED
        </div>
      ) : null}
    </AbsoluteFill>
  );
};

const SecretCompromisedFlash: React.FC<{ start: number; end: number }> = ({ start, end }) => {
  const frame = useCurrentFrame();
  const active = frame >= start && frame < end;
  const opacity = active
    ? interpolate(frame, [start, start + 4, end - 5, end], [0, 1, 1, 0], {
        extrapolateLeft: "clamp",
        extrapolateRight: "clamp",
      })
    : 0;
  const jitter = active ? ((frame * 47) % 33) - 16 : 0;

  return (
    <AbsoluteFill
      style={{
        opacity,
        alignItems: "center",
        justifyContent: "center",
        background: "rgba(216, 58, 47, 0.18)",
      }}
    >
      <div
        style={{
          width: "100%",
          padding: "42px 0",
          background: red,
          color: black,
          fontFamily: display,
          fontSize: 122,
          fontWeight: 800,
          letterSpacing: "0.035em",
          lineHeight: 0.88,
          textAlign: "center",
          textTransform: "uppercase",
          transform: `translateX(${jitter}px) skewX(-5deg)`,
          boxShadow: "0 0 80px rgba(216, 58, 47, 0.58)",
        }}
      >
        SECRETS COMPROMISED
      </div>
    </AbsoluteFill>
  );
};

const StatementScreen: React.FC<{
  headline: string;
  supportLines: string[];
  headlineSize?: number;
  supportSize?: number;
}> = ({ headline, supportLines, headlineSize = 82, supportSize = 30 }) => {
  const frame = useCurrentFrame();
  const { fps } = useVideoConfig();
  const pop = spring({
    frame: frame - 10,
    fps,
    config: { damping: 13, stiffness: 260, mass: 0.7 },
  });
  const lines = supportLines.map((text, index) => ({
    text,
    start: 44 + index * 34,
  }));

  return (
    <AbsoluteFill
      style={{
        background: deepBlack,
        alignItems: "center",
        justifyContent: "center",
      }}
    >
      <SiteBackground gridOpacity={0.52} vignetteOpacity={0.7} />
      <Img
        src={staticFile("icon.png")}
        style={{
          width: 250,
          height: 250,
          objectFit: "contain",
          opacity: interpolate(frame, [0, 14], [0, 1], { extrapolateRight: "clamp" }),
          transform: "translateY(-92px)",
          filter: "drop-shadow(0 0 34px rgba(216, 58, 47, 0.34))",
        }}
      />
      <div
        style={{
          position: "absolute",
          top: 610,
          width: 1420,
          color: ink,
          fontFamily: display,
          fontSize: headlineSize,
          fontWeight: 800,
          letterSpacing: "0.035em",
          lineHeight: 0.9,
          textAlign: "center",
          textTransform: "uppercase",
          opacity: interpolate(frame, [8, 16], [0, 1], { extrapolateRight: "clamp" }),
          transform: `scale(${interpolate(pop, [0, 1], [0.82, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
          })})`,
          textShadow: "0 10px 28px rgba(0, 0, 0, 0.72)",
        }}
      >
        {headline}
      </div>
      <div
        style={{
          position: "absolute",
          top: 724,
          width: 1500,
          color: inkMuted,
          fontFamily: mono,
          fontSize: supportSize,
          fontWeight: 700,
          letterSpacing: 0,
          lineHeight: 1.42,
          textAlign: "center",
          textShadow: "0 8px 18px rgba(0, 0, 0, 0.72)",
        }}
      >
        {lines.map((line) => {
          const opacity = interpolate(frame, [line.start, line.start + 14], [0, 1], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          });
          const y = interpolate(frame, [line.start, line.start + 14], [14, 0], {
            extrapolateLeft: "clamp",
            extrapolateRight: "clamp",
            easing: Easing.bezier(0.16, 1, 0.3, 1),
          });

          return (
            <div
              key={line.text}
              style={{
                opacity,
                transform: `translateY(${y}px)`,
              }}
            >
              {line.text}
            </div>
          );
        })}
      </div>
    </AbsoluteFill>
  );
};

const SecretSafeScreen: React.FC = () => (
  <StatementScreen
    headline="Agents Cannot Access Secrets"
    supportLines={[
      "* Works with any agent",
      "* Even desktop apps like Codex!",
      "* AND it's zeroconf!",
    ]}
  />
);

const SensitiveExecutionScreen: React.FC = () => (
  <StatementScreen
    headline="Sensitive Executions Require Approval"
    headlineSize={72}
    supportSize={28}
    supportLines={[
      "* we only gate what matters",
      "* there's no bypassing our gates: we patched the sources",
    ]}
  />
);

const TokenUseCaseExpanded: React.FC = () => {
  const frame = useCurrentFrame();
  const labelPause = sec(0.45);
  const beforeEnd = sec(4.25) + labelPause;
  const withStart = beforeEnd;
  const finalStart = sec(9.7) + labelPause * 2;
  const beforeActionFrame = frame - labelPause;
  const withFrame = frame - withStart;
  const withActionFrame = withFrame - labelPause;
  const clicked = withActionFrame >= sec(4.25);
  const modalStart = sec(3.1);
  const beforeOpacity =
    frame < beforeEnd
      ? interpolate(frame, [0, 12, beforeEnd - 8, beforeEnd], [1, 1, 1, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        })
      : 0;
  const withOpacity =
    frame >= withStart && frame < finalStart
      ? interpolate(frame, [withStart, withStart + 12, finalStart - 8, finalStart], [0, 1, 1, 0], {
          extrapolateLeft: "clamp",
          extrapolateRight: "clamp",
        })
      : 0;

  return (
    <AbsoluteFill style={{ background: deepBlack }}>
      <SectionBackground />
      <AbsoluteFill style={{ opacity: beforeOpacity }}>
        <ScreenLabel text="Before Automic Vault" />
        <div
          style={{
            opacity: interpolate(beforeActionFrame, [8, 24], [0, 1], {
              extrapolateRight: "clamp",
            }),
          }}
        >
          <TerminalPanel
            command="codex: running: gh auth token"
            response={
              beforeActionFrame >= sec(2.15) ? "gho_x7v9zq2a8f0c1e4b6d3n5p" : undefined
            }
            responseColor={red}
            typedFrames={1.6}
            prompt=""
          />
        </div>
        <SecretCompromisedFlash start={sec(2.9) + labelPause} end={sec(4.08) + labelPause} />
      </AbsoluteFill>
      <AbsoluteFill style={{ opacity: withOpacity }}>
        <ScreenLabel text="With Automic Vault" />
        <Sequence from={withStart} durationInFrames={finalStart - withStart}>
          <div
            style={{
              opacity: interpolate(withActionFrame, [8, 24], [0, 1], {
                extrapolateRight: "clamp",
              }),
            }}
          >
            <TerminalPanel
              command="codex: running: gh auth token"
              response={withActionFrame >= sec(2.35) ? "HUMAN APPROVAL REQUIRED" : undefined}
              responseColor={red}
              typedFrames={1.6}
              prompt=""
            />
          </div>
          {withActionFrame >= modalStart ? (
            <ApprovalModal
              title="Agent wants secret"
              decision="DENY"
              clickFrame={sec(4.25) + labelPause}
              clicked={clicked}
            />
          ) : null}
          {clicked ? (
            <div
              style={{
                position: "absolute",
                right: 172,
                bottom: 155,
                color: red,
                fontFamily: mono,
                fontSize: 36,
                fontWeight: 900,
                textTransform: "uppercase",
                textShadow: "0 0 28px rgba(216, 58, 47, 0.42)",
              }}
            >
              REJECTED
            </div>
          ) : null}
        </Sequence>
      </AbsoluteFill>
      {frame >= finalStart ? (
        <Sequence from={finalStart} durationInFrames={tokenUseCaseDuration - finalStart}>
          <SecretSafeScreen />
        </Sequence>
      ) : null}
    </AbsoluteFill>
  );
};

export const MyComposition = () => {
  return (
    <AbsoluteFill style={{ background: deepBlack }}>
      <Sequence durationInFrames={storyDuration}>
        <Story />
      </Sequence>
      <Sequence from={headlineStart} durationInFrames={headlineDuration}>
        <Headline />
      </Sequence>
      <Sequence from={logoStart} durationInFrames={logoDuration}>
        <LogoHold />
      </Sequence>
      <Sequence from={logoStart - sec(0.16)} durationInFrames={sec(0.74)}>
        <RequiredFadeOut />
      </Sequence>
      <Sequence from={logoStart - sec(0.16)} durationInFrames={sec(0.56)}>
        <HeadlineToLogoTransition />
      </Sequence>
      <Sequence from={useCasesStart - sec(0.16)} durationInFrames={sec(0.56)}>
        <HeadlineToLogoTransition />
      </Sequence>
      <Sequence from={useCasesStart} durationInFrames={tokenUseCaseDuration}>
        <TokenUseCaseExpanded />
      </Sequence>
      <Sequence from={publishStart} durationInFrames={standardUseCaseDuration}>
        <UseCase kind="publish" />
      </Sequence>
      <Sequence from={publishStatementStart} durationInFrames={statementScreenDuration}>
        <SensitiveExecutionScreen />
      </Sequence>
      <Sequence from={installStart} durationInFrames={standardUseCaseDuration}>
        <UseCase kind="install" />
      </Sequence>
      <Sequence from={closeStart} durationInFrames={sec(2)}>
        <LogoHold close />
      </Sequence>
    </AbsoluteFill>
  );
};
