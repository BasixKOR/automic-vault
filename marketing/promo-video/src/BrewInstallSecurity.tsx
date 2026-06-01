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

const fps = 30;
const sec = (value: number) => Math.round(value * fps);

const notificationDuration = sec(2.45);
const actOneDuration = notificationDuration * 4;
const actTwoStart = actOneDuration + sec(0.25);
const actTwoDuration = sec(4.2);
const actThreeStart = actTwoStart + actTwoDuration;
const actThreeDuration = sec(4.8);

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
const emoji = '"Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif';

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

const ToolMist: React.FC<{ visibleFrom: number; visibleTo: number }> = ({ visibleFrom, visibleTo }) => {
  const frame = useCurrentFrame();
  const opacity = fade(frame, visibleFrom, visibleFrom + 18) * fadeOut(frame, visibleTo - 18, visibleTo);
  const tools = ["gh", "awscli", "node", "gemini-cli", "docker", "git"];

  return (
    <AbsoluteFill style={{ opacity }}>
      {tools.map((tool, index) => {
        const x = [198, 1420, 320, 1320, 1040, 620][index];
        const y = [178, 214, 742, 726, 136, 878][index];
        const drift = interpolate(frame + index * 11, [0, brewInstallSecurityDurationInFrames], [0, 46], clamp);

        return (
          <div
            key={tool}
            style={{
              position: "absolute",
              left: x,
              top: y,
              padding: "12px 22px",
              borderRadius: 999,
              border: "1px solid rgba(255,255,255,0.62)",
              background: "rgba(255,255,255,0.34)",
              boxShadow: "0 20px 60px rgba(17,24,39,0.08)",
              color: "rgba(17,24,39,0.32)",
              fontFamily: mono,
              fontSize: 24,
              fontWeight: 760,
              letterSpacing: 0,
              transform: `translateY(${Math.sin((frame + index * 19) / 34) * 10 + drift * 0.22}px)`,
              backdropFilter: "blur(20px)",
              WebkitBackdropFilter: "blur(20px)",
            }}
          >
            {tool}
          </div>
        );
      })}
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

const NotificationScene: React.FC<{ item: Notification; index: number }> = ({ item, index }) => {
  const frame = useCurrentFrame();
  const { fps: configFps } = useVideoConfig();
  const local = frame;
  const card = spring({
    frame: local,
    fps: configFps,
    config: { damping: 17, stiffness: 190, mass: 0.72 },
  });
  const emojiPop = spring({
    frame: local - 10,
    fps: configFps,
    config: { damping: 10, stiffness: 430, mass: 0.42 },
  });
  const entrance = fade(local, 0, 16);
  const exit = fadeOut(local, notificationDuration - 20, notificationDuration - 6);
  const opacity = entrance * exit;
  const y = interpolate(card, [0, 1], [66, 0], clamp);
  const scale = interpolate(card, [0, 1], [0.92, 1], clamp);
  const tilt = interpolate(card, [0, 1], [2.6, 0], clamp);
  const emojiScale = interpolate(emojiPop, [0, 1], [0.2, 1], clamp);
  const emojiRotate = interpolate(emojiPop, [0, 1], [-20, 0], clamp);
  const pulse = interpolate(local % 36, [0, 18, 36], [0.16, 0.44, 0.16], clamp);
  const titleOpacity = fade(local, 18, 34);
  const subOpacity = fade(local, 32, 48);

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
          transform: `translateY(${y}px) scale(${scale}) rotate(${tilt}deg)`,
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
            gap: 34,
            alignItems: "center",
            padding: "44px 54px",
          }}
        >
          <div
            style={{
              position: "relative",
              width: 138,
              height: 138,
              borderRadius: 34,
              display: "flex",
              alignItems: "center",
              justifyContent: "center",
              background: `linear-gradient(145deg, rgba(255,255,255,0.92), ${item.accent}18)`,
              border: `1px solid ${item.accent}32`,
              boxShadow: `0 26px 70px ${item.accent}24, inset 0 1px 0 rgba(255,255,255,0.86)`,
            }}
          >
            <div
              style={{
                position: "absolute",
                inset: -18,
                borderRadius: 44,
                border: `2px solid ${item.accent}`,
                opacity: pulse,
                transform: `scale(${interpolate(local % 36, [0, 36], [0.76, 1.18], clamp)})`,
              }}
            />
            <div
              style={{
                fontFamily: emoji,
                fontSize: 76,
                lineHeight: 1,
                transform: `scale(${emojiScale}) rotate(${emojiRotate}deg)`,
                filter: "drop-shadow(0 12px 24px rgba(17,24,39,0.18))",
              }}
            >
              🚨
            </div>
          </div>
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
              {item.label}
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
                opacity: titleOpacity,
                transform: `translateY(${interpolate(titleOpacity, [0, 1], [22, 0], clamp)}px)`,
              }}
            >
              {item.title}
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
                opacity: subOpacity,
                transform: `translateY(${interpolate(subOpacity, [0, 1], [18, 0], clamp)}px)`,
              }}
            >
              <span>{item.prefix}</span>
              <CodePill tool={item.tool} accent={item.accent} />
              {item.suffix ? <span style={{ marginLeft: 8 }}>{item.suffix}</span> : null}
            </div>
          </div>
          <div
            style={{
              position: "absolute",
              right: 34,
              top: 34,
              display: "flex",
              gap: 8,
            }}
          >
            {[red, amber, green].map((color) => (
              <div
                key={`${color}-${index}`}
                style={{
                  width: 11,
                  height: 11,
                  borderRadius: 11,
                  background: color,
                  opacity: 0.78,
                }}
              />
            ))}
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
  const { fps: configFps } = useVideoConfig();
  const pop = spring({
    frame: local - delay,
    fps: configFps,
    config: { damping: 12, stiffness: 250, mass: 0.64 },
  });
  const opacity = fade(local, delay, delay + 12);

  return (
    <span
      style={{
        display: "inline-block",
        color: accent,
        opacity,
        transform: `translateY(${interpolate(pop, [0, 1], [44, 0], clamp)}px) scale(${interpolate(pop, [0, 1], [0.9, 1], clamp)})`,
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
  const codePop = spring({
    frame: local - 70,
    fps,
    config: { damping: 12, stiffness: 220, mass: 0.72 },
  });

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
        <span> </span>
        <ActTwoWord local={local} delay={46}>
          you
        </ActTwoWord>
        <div
          style={{
            marginTop: 34,
            opacity: fade(local, 68, 86),
            transform: `translateY(${interpolate(codePop, [0, 1], [36, 0], clamp)}) scale(${interpolate(
              codePop,
              [0, 1],
              [0.86, 1],
              clamp,
            )})`,
          }}
        >
          <CodePill tool="brew install" accent={red} large />
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
  const { fps: configFps } = useVideoConfig();
  const logoPop = spring({
    frame: local - 28,
    fps: configFps,
    config: { damping: 14, stiffness: 200, mass: 0.8 },
  });
  const eyebrow = fade(local, 8, 30);
  const logo = fade(local, 28, 52);
  const url = fade(local, 76, 100);

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
          transform: `translateY(${interpolate(eyebrow, [0, 1], [18, 0], clamp)}px)`,
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
          transform: `scale(${interpolate(logoPop, [0, 1], [0.82, 1], clamp)})`,
          filter: "drop-shadow(0 34px 62px rgba(17,24,39,0.18))",
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
        <div
          style={{
            display: "flex",
            flexDirection: "column",
            gap: 4,
          }}
        >
          <div
            style={{
              color: ink,
              fontFamily: sans,
              fontSize: 92,
              fontWeight: 860,
              letterSpacing: 0,
              lineHeight: 0.94,
            }}
          >
            Automic
          </div>
          <div
            style={{
              color: red,
              fontFamily: sans,
              fontSize: 92,
              fontWeight: 860,
              letterSpacing: 0,
              lineHeight: 0.94,
            }}
          >
            Vault
          </div>
        </div>
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
          transform: `translateY(${interpolate(url, [0, 1], [26, 0], clamp)})`,
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
      <ToolMist visibleFrom={0} visibleTo={actTwoStart} />
      {notifications.map((item, index) => (
        <Sequence
          key={item.tool}
          from={index * notificationDuration}
          durationInFrames={notificationDuration}
        >
          <NotificationScene item={item} index={index} />
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
