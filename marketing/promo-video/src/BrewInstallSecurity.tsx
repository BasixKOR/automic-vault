import {
  AbsoluteFill,
  Easing,
  Sequence,
  interpolate,
  useCurrentFrame,
} from "remotion";

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

const ghStoryDuration = sec(26);
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
const leakedToken = "gho_x7v9zq2a8f0c1e4b6d3n5p";

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
          right: 18,
          bottom: -46,
          width: 34,
          height: 34,
          opacity: cursorOpacity,
          transform: `translate(${softY(local, clickStart - 16, clickStart - 6, 24)}px, ${softY(
            local,
            clickStart - 16,
            clickStart - 6,
            22,
          )}px)`,
        }}
      >
        <div
          style={{
            width: 0,
            height: 0,
            borderLeft: "18px solid white",
            borderTop: "14px solid transparent",
            borderBottom: "14px solid transparent",
            filter: "drop-shadow(0 6px 12px rgba(17,24,39,0.28))",
            transform: "rotate(38deg)",
          }}
        />
      </div>
    </div>
  );
};

const AppliedStatus: React.FC<{ local: number; start: number }> = ({
  local,
  start,
}) => (
  <div
    style={{
      marginTop: 34,
      color: green,
      fontFamily: sans,
      fontSize: 34,
      fontWeight: 780,
      letterSpacing: 0,
      lineHeight: 1.1,
      opacity: fade(local, start, start + 16),
      filter: `blur(${softBlur(local, start, start + 16, 8)}px)`,
      transform: `translateY(${softY(local, start, start + 16, 16)}px)`,
    }}
  >
    Automic Hardening Applied
  </div>
);

const GhNotificationCard: React.FC<{
  local: number;
  duration: number;
  withButton?: boolean;
}> = ({ local, duration, withButton = false }) => {
  const motion = sceneExit(local, duration);
  const animateText = !withButton;
  const titleStart = 26;
  const messageStart = 58;
  const hardenButtonStart = 44;
  const hardenClickStart = 106;
  const hardeningAppliedStart = hardenClickStart + 14;

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
                  stride={4}
                  duration={16}
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
                    stride={3}
                    duration={14}
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
                    start={messageStart + 18}
                    stride={3}
                    duration={14}
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
          </div>
          {withButton ? (
            <div
              style={{
                flex: "0 0 360px",
                display: "flex",
                alignItems: "center",
                justifyContent: "flex-end",
                paddingTop: 48,
              }}
            >
              <HardenButton
                local={local}
                start={hardenButtonStart}
                clickStart={hardenClickStart}
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
  const approvalStart = 86;

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
                filter: `blur(${softBlur(local, 82, 98, 8)}px)`,
                transform: `translateY(${softY(local, 82, 98, 18)}px)`,
              }}
            >
              {leakedToken}
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
              waiting for human approval...
            </div>
          ) : null}
        </div>
      </div>
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
        Agent wants to see your GitHub token; {" "}
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
            border: "1px solid rgba(17,24,39,0.1)",
            fontFamily: sans,
            fontSize: 23,
            fontWeight: 740,
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

const GhStoryScene: React.FC = () => {
  const frame = useCurrentFrame();
  const firstNotificationStart = 0;
  const firstTerminalStart = sec(6.7);
  const hardenStart = sec(13.45);
  const gatedTerminalStart = sec(19.15);

  return (
    <AbsoluteFill>
      <GhNotificationCard
        local={frame - firstNotificationStart}
        duration={sec(7)}
      />
      <TerminalAttempt
        local={frame - firstTerminalStart}
        duration={sec(6.95)}
      />
      <GhNotificationCard
        local={frame - hardenStart}
        duration={sec(5.95)}
        withButton
      />
      <TerminalAttempt
        local={frame - gatedTerminalStart}
        duration={ghStoryDuration - gatedTerminalStart}
        gated
      />
      <Flash frame={frame} at={firstTerminalStart} />
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

  return (
    <AbsoluteFill
      style={{ alignItems: "center", justifyContent: "center", opacity: exit }}
    >
      <div
        style={{
          width: 1480,
          textAlign: "center",
          fontFamily: sans,
          fontSize: 104,
          fontWeight: 780,
          letterSpacing: 0,
          lineHeight: 1.16,
          color: ink,
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
          top: 720,
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
          top: 786,
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
          bottom: 98,
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
