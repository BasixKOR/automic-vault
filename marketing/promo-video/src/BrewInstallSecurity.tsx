import {
  AbsoluteFill,
  Easing,
  Sequence,
  interpolate,
  useCurrentFrame,
} from "remotion";

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

const secretInterludeDuration = sec(4.4);
const ghStoryDuration = sec(30.5) + secretInterludeDuration;
const actOneDuration = ghStoryDuration;
const actTwoStart = actOneDuration + sec(0.25);
const actTwoDuration = sec(8.6);

export const brewInstallSecurityDurationInFrames = actTwoStart + actTwoDuration;

const ink = "#111827";
const muted = "#667085";
const red = "#d83a2f";
const blue = "#4776f2";
const green = "#128a62";
const amber = "#d98923";
const paper = "#faf9f5";
const glassBorder = "rgba(255, 255, 255, 0.74)";
const sans =
  '"Geist", ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif';
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';

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

const fadeOutEaseOut = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [1, 0], {
    ...clamp,
    easing: easeOut,
  });

const softY = (
  frame: number,
  start: number,
  end: number,
  from: number,
  to = 0,
) =>
  interpolate(frame, [start, end], [from, to], {
    ...clamp,
    easing: easeOut,
  });

const softBlur = (frame: number, start: number, end: number, from: number) =>
  interpolate(frame, [start, end], [from, 0], {
    ...clamp,
    easing: easeOut,
  });

const ghCommand = "gh auth token";
const redactedTokenPrefix = "gho_x7v9";
const redactedTokenTail = "xxxxxxxxxxxxxxxx";

const BackgroundTexture: React.FC = () => {
  const frame = useCurrentFrame();
  const drift = interpolate(
    frame,
    [0, brewInstallSecurityDurationInFrames],
    [0, 1],
    clamp,
  );
  const slowX = interpolate(drift, [0, 1], [-64, 58], clamp);
  const slowY = interpolate(drift, [0, 1], [44, -56], clamp);
  const wash = interpolate(
    frame % 160,
    [0, 80, 160],
    [0.74, 0.96, 0.74],
    clamp,
  );

  return (
    <AbsoluteFill style={{ background: paper, overflow: "hidden" }}>
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(135deg, #fbfaf6 0%, #eef5fb 38%, #fff7f0 70%, #f8fbf4 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          inset: -220,
          opacity: wash,
          filter: "blur(52px)",
          transform: `translate(${slowX}px, ${slowY}px) rotate(-4deg)`,
          background:
            "linear-gradient(115deg, rgba(216,58,47,0.18) 0%, transparent 27%, rgba(71,118,242,0.18) 44%, transparent 63%, rgba(18,138,98,0.16) 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          inset: -180,
          opacity: 0.72,
          filter: "blur(76px)",
          transform: `translate(${-slowX * 0.7}px, ${slowY * 0.55}px) rotate(8deg)`,
          background:
            "linear-gradient(35deg, transparent 0%, rgba(255,255,255,0.46) 18%, rgba(255,179,71,0.18) 42%, transparent 58%, rgba(216,58,47,0.11) 100%)",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18,
          background:
            "repeating-linear-gradient(90deg, rgba(17,24,39,0.06) 0, rgba(17,24,39,0.06) 1px, transparent 1px, transparent 94px), repeating-linear-gradient(180deg, rgba(17,24,39,0.045) 0, rgba(17,24,39,0.045) 1px, transparent 1px, transparent 94px)",
          transform: `translate(${slowX * 0.18}px, ${slowY * 0.18}px)`,
        }}
      />
      <AbsoluteFill
        style={{
          opacity: 0.18,
          mixBlendMode: "multiply",
          background:
            "repeating-radial-gradient(circle at 24% 20%, rgba(17,24,39,0.18) 0, rgba(17,24,39,0.18) 1px, transparent 1px, transparent 7px)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "linear-gradient(180deg, rgba(255,255,255,0.62) 0%, transparent 24%, transparent 70%, rgba(255,255,255,0.72) 100%)",
        }}
      />
    </AbsoluteFill>
  );
};

const revealStyle = (
  frame: number,
  start: number,
  duration = 16,
  y = 14,
  blur = 7,
) => ({
  opacity: fade(frame, start, start + duration),
  filter: `blur(${softBlur(frame, start, start + duration, blur)}px)`,
  transform: `translateY(${softY(frame, start, start + duration, y)}px)`,
});

const RevealWords: React.FC<{
  text: string;
  local: number;
  start: number;
  stride?: number;
  duration?: number;
  y?: number;
  blur?: number;
  gap?: number;
}> = ({
  text,
  local,
  start,
  stride = 4,
  duration = 16,
  y = 14,
  blur = 7,
  gap = 10,
}) => {
  const words = text.split(" ");

  return (
    <>
      {words.map((word, index) => (
        <span
          key={`${word}-${index}`}
          style={{
            display: "inline-block",
            marginRight: index === words.length - 1 ? 0 : gap,
            whiteSpace: "nowrap",
            ...revealStyle(local, start + index * stride, duration, y, blur),
          }}
        >
          {word}
        </span>
      ))}
    </>
  );
};

const NotificationAlarmBurst: React.FC<{ local: number; start: number }> = ({
  local,
  start,
}) => (
  <div
    style={{
      display: "flex",
      alignItems: "center",
      gap: 14,
      height: 58,
      marginTop: 26,
    }}
  >
    {[0, 1, 2].map((index) => {
      const itemStart = start + index * 16;
      const opacity = fade(local, itemStart, itemStart + 7);
      const shakePower = interpolate(
        local,
        [itemStart, itemStart + 6, itemStart + 24],
        [0, 1, 0],
        clamp,
      );
      const x = Math.sin((local - itemStart) * 1.8) * 7 * shakePower;
      const y = Math.cos((local - itemStart) * 2.1) * 3 * shakePower;
      const rotate = Math.sin((local - itemStart) * 2.4) * 7 * shakePower;
      const scale = interpolate(
        local,
        [itemStart, itemStart + 8, itemStart + 24],
        [0.72, 1.08, 1],
        clamp,
      );

      return (
        <span
          key={index}
          style={{
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            width: 52,
            height: 52,
            borderRadius: 16,
            background: `${red}10`,
            border: `1px solid ${red}24`,
            boxShadow: `0 16px 38px ${red}14`,
            fontSize: 32,
            lineHeight: 1,
            opacity,
            transform: `translate(${x}px, ${y}px) rotate(${rotate}deg) scale(${scale})`,
          }}
        >
          🚨
        </span>
      );
    })}
  </div>
);

const MouseCursor: React.FC<{
  color?: string;
  outline?: string;
  shadow?: string;
  size?: number;
}> = ({
  color = "white",
  outline = "rgba(17,24,39,0.72)",
  shadow = "drop-shadow(0 8px 14px rgba(17,24,39,0.28))",
  size = 38,
}) => (
  <svg
    width={size}
    height={Math.round(size * 1.22)}
    viewBox="0 0 32 39"
    style={{
      display: "block",
      filter: shadow,
      overflow: "visible",
    }}
  >
    <path
      d="M3 2 L3 31 L11.7 22.8 L17.2 36.6 L23.2 34.2 L17.6 20.7 L29.4 20.7 Z"
      fill={color}
      stroke={outline}
      strokeLinejoin="round"
      strokeWidth={2}
    />
  </svg>
);

const TokenSimpleFlash: React.FC<{ local: number }> = ({ local }) => {
  const start = 104;
  const opacity = fade(local, start, start + 9);
  const y = softY(local, start, start + 9, 18);
  const scale = interpolate(
    local,
    [start, start + 8, start + 18],
    [0.94, 1.05, 1],
    clamp,
  );

  return (
    <div
      style={{
        position: "absolute",
        left: 0,
        right: 0,
        top: "calc(50% + 350px)",
        display: "flex",
        justifyContent: "center",
        opacity,
        transform: `translateY(${y}px) scale(${scale})`,
      }}
    >
      <div
        style={{
          borderRadius: 999,
          padding: "16px 26px",
          color: ink,
          background: "rgba(255,255,255,0.72)",
          border: `1px solid ${glassBorder}`,
          boxShadow:
            "0 24px 70px rgba(17,24,39,0.14), inset 0 1px 0 rgba(255,255,255,0.86)",
          fontFamily: sans,
          fontSize: 34,
          fontWeight: 820,
          letterSpacing: 0,
          lineHeight: 1,
          backdropFilter: "blur(20px)",
          WebkitBackdropFilter: "blur(20px)",
        }}
      >
        It&apos;s that simple.
      </div>
    </div>
  );
};

const typedText = (
  text: string,
  local: number,
  start: number,
  duration: number,
) => {
  const chars = Math.floor(
    interpolate(local, [start, start + duration], [0, text.length], clamp),
  );
  return text.slice(0, chars);
};

const sceneExit = (local: number, duration: number) => {
  const exitStart = duration - 26;
  const exitEnd = duration - 4;
  return {
    opacity: fade(local, 0, 16) * fadeOutEaseOut(local, exitStart, exitEnd),
    y: softY(local, 0, 24, 34) + softY(local, exitStart, exitEnd, 0, -22),
    blur: Math.max(
      softBlur(local, 0, 24, 14),
      interpolate(local, [exitStart, exitEnd], [0, 10], clamp),
    ),
  };
};

const HardenButton: React.FC<{
  local: number;
  start: number;
  clickStart: number;
}> = ({ local, start, clickStart }) => {
  const progress = fade(local, start, start + 18);
  const click =
    fade(local, clickStart, clickStart + 4) *
    fadeOut(local, clickStart + 12, clickStart + 24);
  const press = interpolate(
    local,
    [clickStart, clickStart + 5, clickStart + 16],
    [1, 0.965, 1],
    clamp,
  );
  const cursorOpacity =
    fade(local, clickStart - 16, clickStart - 6) *
    fadeOut(local, clickStart + 16, clickStart + 28);
  const cursorX = interpolate(
    local,
    [clickStart - 16, clickStart - 6],
    [304, 226],
    clamp,
  );
  const cursorY = interpolate(
    local,
    [clickStart - 16, clickStart - 6],
    [126, 47],
    clamp,
  );

  return (
    <div
      style={{
        position: "relative",
        width: 340,
        height: 86,
        opacity: progress,
        filter: `blur(${softBlur(local, start, start + 18, 8)}px)`,
        transform: `translateY(${softY(local, start, start + 18, 18)}px) scale(${press})`,
      }}
    >
      <div
        style={{
          height: "100%",
          borderRadius: 26,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          gap: 16,
          color: "white",
          background: `linear-gradient(135deg, ${green}, #0f9f6c)`,
          border: "1px solid rgba(255,255,255,0.52)",
          boxShadow:
            "0 24px 62px rgba(18,138,98,0.35), inset 0 1px 0 rgba(255,255,255,0.34)",
          fontFamily: sans,
          fontSize: 34,
          fontWeight: 780,
          letterSpacing: 0,
        }}
      >
        <span
          style={{
            width: 30,
            height: 34,
            background: "rgba(255,255,255,0.96)",
            clipPath:
              "polygon(50% 0%, 88% 17%, 79% 73%, 50% 100%, 21% 73%, 12% 17%)",
          }}
        />
        Harden gh
      </div>
      <div
        style={{
          position: "absolute",
          inset: -10,
          borderRadius: 34,
          border: "3px solid rgba(18,138,98,0.52)",
          opacity: click,
          transform: `scale(${interpolate(click, [0, 1], [0.92, 1.08], clamp)})`,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: cursorX,
          top: cursorY,
          opacity: cursorOpacity,
          zIndex: 3,
          transform: `scale(${interpolate(click, [0, 1], [1, 0.9], clamp)})`,
        }}
      >
        <MouseCursor
          outline="rgba(12,60,45,0.74)"
          shadow="drop-shadow(0 0 2px rgba(255,255,255,0.86)) drop-shadow(0 8px 14px rgba(17,24,39,0.26))"
        />
      </div>
    </div>
  );
};

const HardenButtonBrand: React.FC<{ local: number; start: number }> = ({
  local,
  start,
}) => {
  const opacity = fade(local, start, start + 18);

  return (
    <div
      style={{
        width: 340,
        marginTop: 20,
        color: ink,
        fontFamily: mono,
        fontSize: 22,
        fontWeight: 860,
        letterSpacing: "0.18em",
        lineHeight: 1,
        opacity,
        textAlign: "center",
        textTransform: "uppercase",
        filter: `blur(${softBlur(local, start, start + 18, 6)}px)`,
        transform: `translateY(${softY(local, start, start + 18, 12)}px)`,
      }}
    >
      AUTOMIC VAULT
    </div>
  );
};

const AppliedStatus: React.FC<{ local: number; start: number }> = ({
  local,
  start,
}) => {
  const spinnerEnd = start + sec(1.15);
  const textStart = spinnerEnd + 3;
  const spinnerOpacity =
    fade(local, start, start + 8) *
    fadeOutEaseOut(local, spinnerEnd - 3, spinnerEnd + 5);
  const spinnerRotation = interpolate(
    local,
    [start, spinnerEnd],
    [0, 360],
    clamp,
  );
  const checkOpacity = fade(local, textStart, textStart + 12);

  return (
    <div
      style={{
        position: "relative",
        marginTop: 34,
        height: 44,
        color: green,
        fontFamily: sans,
        fontSize: 34,
        fontWeight: 780,
        letterSpacing: 0,
        lineHeight: 1.1,
        overflow: "hidden",
      }}
    >
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 3,
          width: 32,
          height: 32,
          borderRadius: 999,
          border: `4px solid ${green}24`,
          borderTopColor: green,
          opacity: spinnerOpacity,
          transform: `rotate(${spinnerRotation}deg)`,
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 0,
          top: 0,
          display: "flex",
          alignItems: "center",
          gap: 16,
          whiteSpace: "nowrap",
        }}
      >
        <span
          style={{
            display: "inline-block",
            width: 18,
            height: 30,
            borderRight: `5px solid ${green}`,
            borderBottom: `5px solid ${green}`,
            opacity: checkOpacity,
            filter: `blur(${softBlur(local, textStart, textStart + 12, 6)}px)`,
            transform: `translateY(${softY(local, textStart, textStart + 12, 12)}px) rotate(45deg)`,
          }}
        />
        <span style={{ display: "inline-block", overflow: "hidden" }}>
          <RevealWords
            text="Automic Hardening Applied"
            local={local}
            start={textStart}
            stride={6}
            duration={20}
            y={14}
            blur={7}
            gap={9}
          />
        </span>
      </div>
    </div>
  );
};

const GhNotificationCard: React.FC<{
  local: number;
  duration: number;
  withButton?: boolean;
}> = ({ local, duration, withButton = false }) => {
  const motion = sceneExit(local, duration);
  const animateText = !withButton;
  const titleStart = 32;
  const messageStart = 100;
  const messageSecondLineStart = messageStart + 42;
  const hardenButtonStart = 44;
  const hardenClickStart = 106;
  const hardeningAppliedStart = hardenClickStart + 6;

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: motion.opacity,
      }}
    >
      <div
        style={{
          position: "absolute",
          width: withButton ? 1340 : 1120,
          minHeight: withButton ? 500 : 430,
          borderRadius: 38,
          border: `1px solid ${glassBorder}`,
          background:
            "linear-gradient(145deg, rgba(255,255,255,0.73), rgba(255,255,255,0.43) 48%, rgba(255,255,255,0.67))",
          boxShadow:
            "0 50px 120px rgba(17,24,39,0.18), 0 18px 40px rgba(17,24,39,0.08), inset 0 1px 0 rgba(255,255,255,0.92), inset 0 -1px 0 rgba(255,255,255,0.34)",
          overflow: "hidden",
          transform: `translateY(${motion.y}px)`,
          filter: `blur(${motion.blur}px)`,
          backdropFilter: "blur(34px) saturate(1.32)",
          WebkitBackdropFilter: "blur(34px) saturate(1.32)",
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            opacity: 0.72,
            background:
              "linear-gradient(118deg, rgba(255,255,255,0.88) 0%, transparent 36%, rgba(255,255,255,0.62) 58%, transparent 82%)",
          }}
        />
        <div
          style={{
            position: "absolute",
            left: -80,
            right: -80,
            top: -130,
            height: 240,
            opacity: 0.62,
            filter: "blur(18px)",
            background: `linear-gradient(90deg, transparent, ${red}32, rgba(255,255,255,0.84), transparent)`,
            transform: `translateX(${interpolate(local, [0, duration], [-140, 140], clamp)}px)`,
          }}
        />
        <div
          style={{
            position: "relative",
            zIndex: 1,
            display: "flex",
            height: "100%",
            minHeight: withButton ? 500 : 430,
            alignItems: "center",
            gap: withButton ? 54 : 0,
            padding: "58px 72px",
          }}
        >
          <div style={{ flex: 1, minWidth: 0 }}>
            <div
              style={{
                display: "inline-flex",
                alignItems: "center",
                gap: 10,
                borderRadius: 999,
                padding: "8px 14px",
                background: `${red}13`,
                border: `1px solid ${red}28`,
                color: red,
                fontFamily: mono,
                fontSize: 19,
                fontWeight: 820,
                letterSpacing: "0.04em",
                textTransform: "uppercase",
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: 999,
                  background: red,
                  boxShadow: `0 0 18px ${red}`,
                }}
              />
              secret exposure
            </div>
            <div
              style={{
                color: ink,
                fontFamily: sans,
                fontSize: 62,
                fontWeight: 780,
                letterSpacing: 0,
                lineHeight: 1.02,
                marginTop: 24,
                overflow: "hidden",
              }}
            >
              {animateText ? (
                <RevealWords
                  text="Plain Text Secret Detected"
                  local={local}
                  start={titleStart}
                  stride={8}
                  duration={24}
                  y={20}
                  blur={8}
                  gap={14}
                />
              ) : (
                "Plain Text GitHub Token"
              )}
            </div>
            <div
              style={{
                color: muted,
                fontFamily: sans,
                fontSize: 34,
                fontWeight: 560,
                letterSpacing: 0,
                lineHeight: 1.28,
                marginTop: 22,
                overflow: "hidden",
              }}
            >
              <div style={{ minHeight: 46 }}>
                {animateText ? (
                  <RevealWords
                    text="Your GitHub token is trivially available"
                    local={local}
                    start={messageStart}
                    stride={7}
                    duration={22}
                    y={12}
                    blur={6}
                    gap={8}
                  />
                ) : (
                  "Your GitHub token is trivially available"
                )}
              </div>
              <div style={{ minHeight: 48, marginTop: 4 }}>
                {animateText ? (
                  <RevealWords
                    text="to agents and malware."
                    local={local}
                    start={messageSecondLineStart}
                    stride={7}
                    duration={22}
                    y={12}
                    blur={6}
                    gap={8}
                  />
                ) : (
                  "to agents and malware."
                )}
              </div>
            </div>
            {withButton ? (
              <AppliedStatus local={local} start={hardeningAppliedStart} />
            ) : null}
            {!withButton ? (
              <NotificationAlarmBurst local={local} start={176} />
            ) : null}
          </div>
          {withButton ? (
            <div
              style={{
                flex: "0 0 360px",
                display: "flex",
                flexDirection: "column",
                alignItems: "flex-end",
                justifyContent: "center",
                paddingTop: 48,
              }}
            >
              <HardenButton
                local={local}
                start={hardenButtonStart}
                clickStart={hardenClickStart}
              />
              <HardenButtonBrand
                local={local}
                start={hardenButtonStart + 18}
              />
            </div>
          ) : null}
        </div>
      </div>
    </AbsoluteFill>
  );
};

const TerminalAttempt: React.FC<{
  local: number;
  duration: number;
  gated?: boolean;
}> = ({ local, duration, gated = false }) => {
  const motion = sceneExit(local, duration);
  const typeStart = 32;
  const typeDuration = 32;
  const command = typedText(ghCommand, local, typeStart, typeDuration);
  const commandComplete = local >= typeStart + typeDuration;
  const tokenOpacity = gated ? 0 : fade(local, 82, 98);
  const tokenBlur = Math.max(12, softBlur(local, 82, 98, 8));
  const approvalStart = 92;

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: motion.opacity,
      }}
    >
      <div
        style={{
          position: "absolute",
          width: 1160,
          height: 610,
          borderRadius: 28,
          overflow: "hidden",
          background:
            "linear-gradient(180deg, rgba(22,29,39,0.96), rgba(8,12,18,0.97))",
          border: "1px solid rgba(255,255,255,0.13)",
          boxShadow:
            "0 44px 110px rgba(17,24,39,0.34), inset 0 1px 0 rgba(255,255,255,0.08)",
          transform: `translateY(${motion.y}px)`,
          filter: `blur(${motion.blur}px)`,
        }}
      >
        <div
          style={{
            height: 66,
            display: "flex",
            alignItems: "center",
            justifyContent: "space-between",
            padding: "0 28px",
            background: "rgba(255,255,255,0.05)",
            borderBottom: "1px solid rgba(255,255,255,0.09)",
            color: "rgba(255,255,255,0.58)",
            fontFamily: mono,
            fontSize: 18,
            fontWeight: 760,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
          }}
        >
          <span>agent terminal</span>
          <span>{gated ? "after hardening" : "before hardening"}</span>
        </div>
        <div
          style={{
            padding: "54px 60px",
            color: "#e8eef7",
            fontFamily: mono,
            fontSize: 42,
            lineHeight: 1.45,
            fontWeight: 680,
          }}
        >
          <div>
            <span style={{ color: amber }}>$ </span>
            {command}
            <span
              style={{ opacity: local % 18 < 9 && !commandComplete ? 1 : 0 }}
            >
              _
            </span>
          </div>
          {!gated ? (
            <div
              style={{
                marginTop: 44,
                color: red,
                fontSize: 40,
                opacity: tokenOpacity,
                transform: `translateY(${softY(local, 82, 98, 18)}px)`,
                textShadow: `0 0 22px ${red}aa`,
              }}
            >
              <span>{redactedTokenPrefix}</span>
              <span style={{ filter: `blur(${tokenBlur}px)` }}>
                {redactedTokenTail}
              </span>
            </div>
          ) : null}
          {gated && commandComplete ? (
            <div
              style={{
                marginTop: 46,
                color: "rgba(232,238,247,0.45)",
                fontSize: 31,
                opacity: fade(local, 70, 82),
              }}
            >
              human approval required…
            </div>
          ) : null}
        </div>
      </div>
      {!gated ? <TokenSimpleFlash local={local} /> : null}
      {gated ? <ApprovalWindow local={local} start={approvalStart} /> : null}
    </AbsoluteFill>
  );
};

const ApprovalWindow: React.FC<{ local: number; start: number }> = ({
  local,
  start,
}) => {
  const opacity = fade(local, start, start + 16);
  const y = softY(local, start, start + 16, 24);
  const blur = softBlur(local, start, start + 16, 10);
  const cursorStart = start + 52;
  const denyClickStart = start + 70;
  const cursorOpacity =
    fade(local, cursorStart, cursorStart + 10) *
    fadeOut(local, denyClickStart + 18, denyClickStart + 30);
  const cursorX = interpolate(
    local,
    [cursorStart, denyClickStart - 5],
    [536, 420],
    clamp,
  );
  const cursorY = interpolate(
    local,
    [cursorStart, denyClickStart - 5],
    [358, 340],
    clamp,
  );
  const denyClick =
    fade(local, denyClickStart, denyClickStart + 4) *
    fadeOut(local, denyClickStart + 11, denyClickStart + 22);
  const denyPress = interpolate(
    local,
    [denyClickStart, denyClickStart + 5, denyClickStart + 14],
    [1, 0.965, 1],
    clamp,
  );

  return (
    <div
      style={{
        position: "absolute",
        width: 640,
        minHeight: 350,
        borderRadius: 34,
        padding: "44px 46px",
        color: ink,
        background:
          "linear-gradient(145deg, rgba(255,255,255,0.86), rgba(255,255,255,0.64))",
        border: `1px solid ${glassBorder}`,
        boxShadow:
          "0 42px 110px rgba(17,24,39,0.26), inset 0 1px 0 rgba(255,255,255,0.96)",
        opacity,
        filter: `blur(${blur}px)`,
        transform: `translateY(${y}px)`,
        backdropFilter: "blur(30px) saturate(1.24)",
        WebkitBackdropFilter: "blur(30px) saturate(1.24)",
      }}
    >
      <div
        style={{
          display: "inline-flex",
          alignItems: "center",
          gap: 10,
          borderRadius: 999,
          padding: "8px 13px",
          color: green,
          background: `${green}12`,
          border: `1px solid ${green}26`,
          fontFamily: mono,
          fontSize: 17,
          fontWeight: 820,
          letterSpacing: "0.05em",
          textTransform: "uppercase",
        }}
      >
        human gate
      </div>
      <div
        style={{
          marginTop: 24,
          fontFamily: sans,
          fontSize: 45,
          fontWeight: 790,
          letterSpacing: 0,
          lineHeight: 1.05,
        }}
      >
        Human Approval Required
      </div>
      <div
        style={{
          marginTop: 18,
          color: muted,
          fontFamily: sans,
          fontSize: 25,
          fontWeight: 560,
          lineHeight: 1.32,
        }}
      >
        Agent wants to see your GitHub token;{" "}
        <span style={{ color: ink, fontFamily: mono, fontWeight: 780 }}>
          {ghCommand}
        </span>
        .
      </div>
      <div
        style={{
          display: "flex",
          justifyContent: "flex-end",
          gap: 16,
          marginTop: 34,
        }}
      >
        <div
          style={{
            borderRadius: 18,
            padding: "15px 25px",
            color: muted,
            background: "rgba(255,255,255,0.66)",
            border: `1px solid rgba(216,58,47,${interpolate(
              denyClick,
              [0, 1],
              [0.12, 0.42],
              clamp,
            )})`,
            fontFamily: sans,
            fontSize: 23,
            fontWeight: 740,
            boxShadow: `0 0 0 ${interpolate(
              denyClick,
              [0, 1],
              [0, 5],
              clamp,
            )}px rgba(216,58,47,0.13)`,
            transform: `scale(${denyPress})`,
          }}
        >
          Deny
        </div>
        <div
          style={{
            borderRadius: 18,
            padding: "15px 25px",
            color: "white",
            background: green,
            border: "1px solid rgba(255,255,255,0.36)",
            fontFamily: sans,
            fontSize: 23,
            fontWeight: 740,
            boxShadow: "0 18px 42px rgba(18,138,98,0.28)",
          }}
        >
          Allow
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: cursorX,
          top: cursorY,
          width: 34,
          height: 34,
          opacity: cursorOpacity,
          zIndex: 3,
          transform: `scale(${interpolate(
            denyClick,
            [0, 1],
            [1, 0.9],
            clamp,
          )})`,
        }}
      >
        <MouseCursor
          color={ink}
          outline="rgba(255,255,255,0.88)"
          shadow="drop-shadow(0 0 2px rgba(255,255,255,0.9)) drop-shadow(0 8px 14px rgba(17,24,39,0.3))"
        />
      </div>
    </div>
  );
};

const Flash: React.FC<{ frame: number; at: number }> = ({ frame, at }) => {
  const flashIn = fade(frame, at - 3, at);
  const flashOut = fadeOut(frame, at + 2, at + 12);
  const opacity = flashIn * flashOut * 0.62;

  return (
    <AbsoluteFill
      style={{
        background: "white",
        opacity,
        pointerEvents: "none",
      }}
    />
  );
};

const SecretInterlude: React.FC<{ local: number; duration: number }> = ({
  local,
  duration,
}) => {
  const motion = sceneExit(local, duration);
  const badge = fade(local, 10, 24);
  const rule = fade(local, 28, 42);
  const words = [
    { text: "keep", italic: false },
    { text: "secrets", italic: false },
    { text: "secret", italic: true },
  ];

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "center",
        opacity: motion.opacity,
      }}
    >
      <div
        style={{
          position: "absolute",
          width: 1480,
          textAlign: "center",
          transform: `translateY(${motion.y}px)`,
          filter: `blur(${motion.blur}px)`,
        }}
      >
        <div
          style={{
            display: "inline-flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 12,
            borderRadius: 999,
            padding: "12px 20px",
            color: green,
            background: `${green}12`,
            border: `1px solid ${green}2b`,
            boxShadow: `0 18px 54px ${green}14`,
            fontFamily: mono,
            fontSize: 27,
            fontWeight: 860,
            letterSpacing: "0.14em",
            lineHeight: 1,
            opacity: badge,
            textTransform: "uppercase",
            transform: `translateY(${softY(local, 10, 24, 14)}px)`,
          }}
        >
          <span
            style={{
              width: 15,
              height: 18,
              background: green,
              clipPath:
                "polygon(50% 0%, 88% 17%, 79% 73%, 50% 100%, 21% 73%, 12% 17%)",
            }}
          />
          Automic Vault
        </div>
        <div
          style={{
            marginTop: 36,
            color: ink,
            fontFamily: sans,
            fontSize: 112,
            fontWeight: 820,
            letterSpacing: 0,
            lineHeight: 1.02,
          }}
        >
          {words.map((word, index) => (
            <span
              key={word.text}
              style={{
                display: "inline-block",
                fontStyle: word.italic ? "italic" : "normal",
                marginRight: index === words.length - 1 ? 0 : 24,
                ...revealStyle(local, 14 + index * 5, 16, 24, 10),
              }}
            >
              {word.text}
            </span>
          ))}
        </div>
        <div
          style={{
            width: 760,
            height: 8,
            margin: "42px auto 0",
            borderRadius: 999,
            background: `linear-gradient(90deg, transparent, ${green}, ${blue}, ${green}, transparent)`,
            opacity: rule,
            transform: `scaleX(${interpolate(rule, [0, 1], [0.28, 1], clamp)})`,
            boxShadow: `0 20px 64px ${green}20`,
          }}
        />
      </div>
    </AbsoluteFill>
  );
};

const GhStoryScene: React.FC = () => {
  const frame = useCurrentFrame();
  const firstNotificationStart = 0;
  const firstTerminalStart = sec(8.5);
  const secretInterludeStart = sec(15.65);
  const hardenStart = secretInterludeStart + secretInterludeDuration;
  const gatedTerminalStart = sec(23.7) + secretInterludeDuration;

  return (
    <AbsoluteFill>
      <GhNotificationCard
        local={frame - firstNotificationStart}
        duration={sec(8.75)}
      />
      <TerminalAttempt local={frame - firstTerminalStart} duration={sec(7.3)} />
      <SecretInterlude
        local={frame - secretInterludeStart}
        duration={secretInterludeDuration}
      />
      <GhNotificationCard
        local={frame - hardenStart}
        duration={sec(8.2)}
        withButton
      />
      <TerminalAttempt
        local={frame - gatedTerminalStart}
        duration={ghStoryDuration - gatedTerminalStart}
        gated
      />
      <Flash frame={frame} at={firstTerminalStart} />
      <Flash frame={frame} at={secretInterludeStart} />
      <Flash frame={frame} at={hardenStart} />
      <Flash frame={frame} at={gatedTerminalStart} />
    </AbsoluteFill>
  );
};

const ActTwoWord: React.FC<{
  children: string;
  local: number;
  delay: number;
  accent?: string;
}> = ({ children, local, delay, accent = ink }) => {
  const opacity = fade(local, delay, delay + 18);
  const y = softY(local, delay, delay + 18, 18);
  const blur = softBlur(local, delay, delay + 18, 8);

  return (
    <span
      style={{
        display: "inline-block",
        color: accent,
        opacity,
        filter: `blur(${blur}px)`,
        transform: `translateY(${y}px)`,
        whiteSpace: "nowrap",
      }}
    >
      {children}
    </span>
  );
};

const BrewInstallScene: React.FC = () => {
  const frame = useCurrentFrame();
  const local = frame;
  const exit = fadeOut(local, actTwoDuration - 18, actTwoDuration);
  const marker = fade(local, 92, 118);
  const commandOpacity = fade(local, 58, 76);
  const commandY = softY(local, 58, 76, 22);
  const commandBlur = softBlur(local, 58, 76, 10);
  const eyebrowStart = 128;
  const eyebrow = fade(local, eyebrowStart, eyebrowStart + 22);
  const urlStart = 166;
  const url = fade(local, urlStart, urlStart + 24);
  const urlY = softY(local, urlStart, urlStart + 24, 18);
  const urlBlur = softBlur(local, urlStart, urlStart + 24, 8);
  const finalStackTop = 252;
  const markerTop = finalStackTop + 316;
  const eyebrowTop = finalStackTop + 382;
  const urlTop = finalStackTop + 470;

  return (
    <AbsoluteFill
      style={{
        alignItems: "center",
        justifyContent: "flex-start",
        opacity: exit,
      }}
    >
      <div
        style={{
          position: "absolute",
          left: "50%",
          top: finalStackTop,
          width: 1480,
          textAlign: "center",
          fontFamily: sans,
          fontSize: 104,
          fontWeight: 780,
          letterSpacing: 0,
          lineHeight: 1.16,
          color: ink,
          transform: "translateX(-50%)",
        }}
      >
        <div
          style={{
            minHeight: 122,
          }}
        >
          <ActTwoWord local={local} delay={4}>
            Secure
          </ActTwoWord>
          <span> </span>
          <ActTwoWord local={local} delay={18}>
            the
          </ActTwoWord>
          <span> </span>
          <ActTwoWord local={local} delay={32}>
            tools
          </ActTwoWord>
        </div>
        <div
          style={{
            minHeight: 118,
            marginTop: 22,
            display: "flex",
            alignItems: "center",
            justifyContent: "center",
            gap: 28,
          }}
        >
          <ActTwoWord local={local} delay={46}>
            you
          </ActTwoWord>
          <div
            style={{
              display: "inline-flex",
              alignItems: "center",
              justifyContent: "flex-start",
              flex: "0 0 auto",
              width: 610,
              borderRadius: 22,
              padding: "18px 26px",
              border: `1px solid ${red}2f`,
              background: "rgba(255,255,255,0.66)",
              boxShadow: `0 24px 70px ${red}24, inset 0 1px 0 rgba(255,255,255,0.88)`,
              color: ink,
              fontFamily: mono,
              fontSize: 80,
              fontWeight: 820,
              lineHeight: 1,
              letterSpacing: 0,
              opacity: commandOpacity,
              filter: `blur(${commandBlur}px)`,
              transform: `translateY(${commandY}px)`,
              overflow: "hidden",
              whiteSpace: "nowrap",
            }}
          >
            <ActTwoWord local={local} delay={60}>
              brew
            </ActTwoWord>
            <span style={{ display: "inline-block", width: 28 }} />
            <ActTwoWord local={local} delay={74}>
              install
            </ActTwoWord>
          </div>
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: "50%",
          top: markerTop,
          width: 740,
          height: 8,
          borderRadius: 999,
          background: `linear-gradient(90deg, transparent, ${red}, ${blue}, ${green}, transparent)`,
          opacity: marker,
          transform: `translateX(-50%) scaleX(${interpolate(marker, [0, 1], [0.24, 1], clamp)})`,
          boxShadow: "0 18px 54px rgba(71,118,242,0.2)",
        }}
      />
      <div
        style={{
          position: "absolute",
          top: eyebrowTop,
          left: 0,
          right: 0,
          color: muted,
          fontFamily: mono,
          fontSize: 25,
          fontWeight: 780,
          letterSpacing: "0.1em",
          textAlign: "center",
          textTransform: "uppercase",
          opacity: eyebrow,
          filter: `blur(${softBlur(local, eyebrowStart, eyebrowStart + 22, 7)}px)`,
          transform: `translateY(${softY(local, eyebrowStart, eyebrowStart + 22, 16)}px)`,
        }}
      >
        from the creator of homebrew
      </div>
      <div
        style={{
          position: "absolute",
          left: "50%",
          top: urlTop,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          minWidth: 780,
          padding: "24px 38px",
          borderRadius: 999,
          border: `1px solid ${glassBorder}`,
          background: "rgba(255,255,255,0.54)",
          boxShadow:
            "0 28px 90px rgba(17,24,39,0.12), inset 0 1px 0 rgba(255,255,255,0.88)",
          color: ink,
          fontFamily: mono,
          fontSize: 38,
          fontWeight: 760,
          letterSpacing: 0,
          opacity: url,
          filter: `blur(${urlBlur}px)`,
          transform: `translateX(-50%) translateY(${urlY}px)`,
          backdropFilter: "blur(24px)",
          WebkitBackdropFilter: "blur(24px)",
        }}
      >
        https://www.automicvault.com
      </div>
    </AbsoluteFill>
  );
};

export const BrewInstallSecurityComposition: React.FC = () => {
  return (
    <AbsoluteFill style={{ background: paper }}>
      <BackgroundTexture />
      <Sequence durationInFrames={actOneDuration}>
        <GhStoryScene />
      </Sequence>
      <Sequence from={actTwoStart} durationInFrames={actTwoDuration}>
        <BrewInstallScene />
      </Sequence>
    </AbsoluteFill>
  );
};
