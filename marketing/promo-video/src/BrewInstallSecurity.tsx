import {
  AbsoluteFill,
  Easing,
  Img,
  Sequence,
  interpolate,
  staticFile,
  useCurrentFrame,
} from "remotion";

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

const notificationDuration = sec(4.1);
const actOneDuration = notificationDuration * 4;
const actTwoStart = actOneDuration + sec(0.25);
const actTwoDuration = sec(5.8);
const actThreeStart = actTwoStart + actTwoDuration;
const actThreeDuration = sec(6.2);

export const brewInstallSecurityDurationInFrames = actThreeStart + actThreeDuration;

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

const softY = (frame: number, start: number, end: number, from: number, to = 0) =>
  interpolate(frame, [start, end], [from, to], {
    ...clamp,
    easing: easeOut,
  });

const softBlur = (frame: number, start: number, end: number, from: number) =>
  interpolate(frame, [start, end], [from, 0], {
    ...clamp,
    easing: easeOut,
  });

type Notification = {
  accent: string;
  label: string;
  title: string;
  prefix: string;
  tool: string;
  suffix?: string;
};

const notifications: Notification[] = [
  {
    accent: red,
    label: "secret scan",
    title: "Plain text secret exposure",
    prefix: "Exposure detected in",
    tool: "gh",
  },
  {
    accent: amber,
    label: "credential hardening",
    title: "Cloud key left readable",
    prefix: "AWS credentials found for",
    tool: "awscli",
  },
  {
    accent: blue,
    label: "install guard",
    title: "Postinstall wants a token",
    prefix: "Script risk flagged in",
    tool: "node",
  },
  {
    accent: green,
    label: "approval gate",
    title: "Agent command needs approval",
    prefix: "Sensitive action gated in",
    tool: "gemini-cli",
  },
];

const BackgroundTexture: React.FC = () => {
  const frame = useCurrentFrame();
  const drift = interpolate(frame, [0, brewInstallSecurityDurationInFrames], [0, 1], clamp);
  const slowX = interpolate(drift, [0, 1], [-64, 58], clamp);
  const slowY = interpolate(drift, [0, 1], [44, -56], clamp);
  const wash = interpolate(frame % 160, [0, 80, 160], [0.74, 0.96, 0.74], clamp);

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

const CodePill: React.FC<{ tool: string; accent: string; large?: boolean }> = ({ tool, accent, large = false }) => (
  <span
    style={{
      display: "inline-flex",
      alignItems: "center",
      justifyContent: "center",
      borderRadius: large ? 22 : 14,
      padding: large ? "18px 26px" : "4px 11px 5px",
      marginLeft: large ? 0 : 8,
      border: `1px solid ${accent}2f`,
      background: large ? "rgba(255,255,255,0.66)" : `${accent}14`,
      boxShadow: large ? `0 24px 70px ${accent}24, inset 0 1px 0 rgba(255,255,255,0.88)` : "none",
      color: large ? ink : accent,
      fontFamily: mono,
      fontSize: large ? 80 : 33,
      fontWeight: large ? 820 : 760,
      lineHeight: 1,
      letterSpacing: 0,
      verticalAlign: "baseline",
      whiteSpace: "nowrap",
    }}
  >
    {large ? "brew install" : `\`${tool}\``}
  </span>
);

const revealStyle = (frame: number, start: number, duration = 16, y = 14, blur = 7) => ({
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
}> = ({ text, local, start, stride = 4, duration = 16, y = 14, blur = 7, gap = 10 }) => {
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

const NotificationScene: React.FC<{ item: Notification }> = ({ item }) => {
  const frame = useCurrentFrame();
  const local = frame;
  const entrance = fade(local, 0, 16);
  const exit = fadeOut(local, notificationDuration - 20, notificationDuration - 6);
  const opacity = entrance * exit;
  const y = softY(local, 0, 24, 34) + softY(local, notificationDuration - 30, notificationDuration - 6, 0, -22);
  const blur = Math.max(
    softBlur(local, 0, 24, 14),
    interpolate(local, [notificationDuration - 30, notificationDuration - 6], [0, 10], clamp),
  );
  const labelMotion = revealStyle(local, 14, 16, 9, 5);
  const titleStart = 26;
  const subtitleStart = 52;
  const prefixWordCount = item.prefix.split(" ").length;
  const toolStart = subtitleStart + prefixWordCount * 3 + 5;
  const suffixStart = toolStart + 9;

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", opacity }}>
      <div
        style={{
          position: "absolute",
          width: 1030,
          minHeight: 370,
          borderRadius: 38,
          border: `1px solid ${glassBorder}`,
          background:
            "linear-gradient(145deg, rgba(255,255,255,0.72), rgba(255,255,255,0.42) 48%, rgba(255,255,255,0.66))",
          boxShadow:
            "0 50px 120px rgba(17,24,39,0.18), 0 18px 40px rgba(17,24,39,0.08), inset 0 1px 0 rgba(255,255,255,0.92), inset 0 -1px 0 rgba(255,255,255,0.34)",
          overflow: "hidden",
          transform: `translateY(${y}px)`,
          filter: `blur(${blur}px)`,
          backdropFilter: "blur(34px) saturate(1.32)",
          WebkitBackdropFilter: "blur(34px) saturate(1.32)",
        }}
      >
        <div
          style={{
            position: "absolute",
            inset: 0,
            opacity: 0.7,
            background:
              "linear-gradient(118deg, rgba(255,255,255,0.86) 0%, transparent 36%, rgba(255,255,255,0.62) 58%, transparent 82%)",
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
            background: `linear-gradient(90deg, transparent, ${item.accent}34, rgba(255,255,255,0.84), transparent)`,
            transform: `translateX(${interpolate(local, [0, notificationDuration], [-140, 140], clamp)}px)`,
          }}
        />
        <div
          style={{
            position: "relative",
            zIndex: 1,
            display: "flex",
            height: "100%",
            minHeight: 370,
            alignItems: "center",
            padding: "52px 68px",
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
                background: `${item.accent}13`,
                border: `1px solid ${item.accent}28`,
                color: item.accent,
                fontFamily: mono,
                fontSize: 19,
                fontWeight: 820,
                letterSpacing: "0.04em",
                textTransform: "uppercase",
                ...labelMotion,
              }}
            >
              <span
                style={{
                  width: 8,
                  height: 8,
                  borderRadius: 999,
                  background: item.accent,
                  boxShadow: `0 0 18px ${item.accent}`,
                }}
              />
              <RevealWords text={item.label} local={local} start={18} stride={2} duration={12} y={5} blur={4} gap={6} />
            </div>
            <div
              style={{
                color: ink,
                fontFamily: sans,
                fontSize: 66,
                fontWeight: 780,
                letterSpacing: 0,
                lineHeight: 1.02,
                marginTop: 24,
                overflow: "hidden",
              }}
            >
              <RevealWords text={item.title} local={local} start={titleStart} stride={4} duration={16} y={20} blur={8} gap={14} />
            </div>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                flexWrap: "wrap",
                color: muted,
                fontFamily: sans,
                fontSize: 36,
                fontWeight: 560,
                letterSpacing: 0,
                lineHeight: 1.25,
                marginTop: 18,
                overflow: "hidden",
              }}
            >
              <RevealWords text={item.prefix} local={local} start={subtitleStart} stride={3} duration={14} y={12} blur={6} gap={8} />
              <span
                style={{
                  display: "inline-flex",
                  ...revealStyle(local, toolStart, 14, 12, 6),
                }}
              >
                <CodePill tool={item.tool} accent={item.accent} />
              </span>
              {item.suffix ? (
                <span style={{ marginLeft: 8 }}>
                  <RevealWords text={item.suffix} local={local} start={suffixStart} stride={3} duration={14} y={12} blur={6} gap={8} />
                </span>
              ) : null}
            </div>
          </div>
        </div>
      </div>
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
  const installProgress = fade(local, 74, 88);
  const commandOpacity = fade(local, 58, 76);
  const commandY = softY(local, 58, 76, 22);
  const commandBlur = softBlur(local, 58, 76, 10);

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center", opacity: exit }}>
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
              width: interpolate(installProgress, [0, 1], [274, 610], clamp),
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
    </AbsoluteFill>
  );
};

const ClosingScene: React.FC = () => {
  const frame = useCurrentFrame();
  const local = frame;
  const eyebrow = fade(local, 8, 30);
  const logo = fade(local, 28, 52);
  const url = fade(local, 76, 100);
  const logoY = softY(local, 28, 52, 24);
  const logoBlur = softBlur(local, 28, 52, 10);
  const urlY = softY(local, 76, 100, 18);
  const urlBlur = softBlur(local, 76, 100, 8);

  return (
    <AbsoluteFill style={{ alignItems: "center", justifyContent: "center" }}>
      <div
        style={{
          position: "absolute",
          top: 190,
          color: muted,
          fontFamily: mono,
          fontSize: 28,
          fontWeight: 780,
          letterSpacing: "0.12em",
          textTransform: "uppercase",
          opacity: eyebrow,
          filter: `blur(${softBlur(local, 8, 30, 7)}px)`,
          transform: `translateY(${softY(local, 8, 30, 18)}px)`,
        }}
      >
        From the creator of Homebrew
      </div>
      <div
        style={{
          position: "absolute",
          top: 302,
          display: "flex",
          alignItems: "center",
          gap: 42,
          opacity: logo,
          filter: `blur(${logoBlur}px) drop-shadow(0 34px 62px rgba(17,24,39,0.18))`,
          transform: `translateY(${logoY}px)`,
        }}
      >
        <Img
          src={staticFile("icon.png")}
          style={{
            width: 228,
            height: 228,
            objectFit: "contain",
          }}
        />
        <Img
          src={staticFile("site-wordmark.webp")}
          style={{
            width: 612,
            height: 158,
            objectFit: "contain",
          }}
        />
      </div>
      <div
        style={{
          position: "absolute",
          top: 695,
          display: "inline-flex",
          alignItems: "center",
          justifyContent: "center",
          minWidth: 780,
          padding: "24px 38px",
          borderRadius: 999,
          border: `1px solid ${glassBorder}`,
          background: "rgba(255,255,255,0.54)",
          boxShadow: "0 28px 90px rgba(17,24,39,0.12), inset 0 1px 0 rgba(255,255,255,0.88)",
          color: ink,
          fontFamily: mono,
          fontSize: 38,
          fontWeight: 760,
          letterSpacing: 0,
          opacity: url,
          filter: `blur(${urlBlur}px)`,
          transform: `translateY(${urlY}px)`,
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
      {notifications.map((item, index) => (
        <Sequence
          key={item.tool}
          from={index * notificationDuration}
          durationInFrames={notificationDuration}
        >
          <NotificationScene item={item} />
        </Sequence>
      ))}
      <Sequence from={actTwoStart} durationInFrames={actTwoDuration}>
        <BrewInstallScene />
      </Sequence>
      <Sequence from={actThreeStart} durationInFrames={actThreeDuration}>
        <ClosingScene />
      </Sequence>
    </AbsoluteFill>
  );
};
