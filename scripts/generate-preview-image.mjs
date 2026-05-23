#!/usr/bin/env node
import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = dirname(fileURLToPath(import.meta.url));
const repoRoot = resolve(__dirname, "..");
const outDir = resolve(repoRoot, "www");
const pngPath = resolve(outDir, "preview.png");
const jpgPath = resolve(outDir, "preview.jpg");
const tempSvgPath = resolve(outDir, ".preview.tmp.svg");

const width = 1573;
const height = 1000;
const logoPath = resolve(repoRoot, "assets", "logo.png");
const logoData = readFileSync(logoPath).toString("base64");

const cream = "#e6dfd3";
const muted = "#b9b3aa";
const red = "#ff3b24";
const green = "#9ccf3f";
const orange = "#ff8a00";
const panel = "#1b1e1d";
const grid = "#343836";

const iconCircle = (cx, cy, color) => `
  <circle cx="${cx}" cy="${cy}" r="24" fill="#191b1a" stroke="#65635d" stroke-width="1.2"/>
  <circle cx="${cx}" cy="${cy}" r="17" fill="none" stroke="${color}" stroke-width="2" opacity="0.28"/>
`;

const stat = ({ x, index, label, value, body, color, icon }) => `
  <g transform="translate(${x} 0)">
    ${iconCircle(25, 0, color)}
    ${icon}
    <text x="72" y="6" class="mono stat-label">${label}</text>
    <text x="276" y="6" class="mono stat-index">.${index}</text>
    <text x="0" y="122" class="stat-value">${value}</text>
    <text x="0" y="170" class="stat-body">${body}</text>
    <line x1="0" y1="204" x2="184" y2="204" stroke="${color}" stroke-width="3"/>
    <line x1="195" y1="204" x2="212" y2="204" stroke="#77736b" stroke-width="3" opacity="0.65"/>
    <line x1="225" y1="204" x2="237" y2="204" stroke="#77736b" stroke-width="3" opacity="0.65"/>
    <path d="M250 196 L258 204 L250 212" fill="none" stroke="${color}" stroke-width="3" stroke-linecap="round" stroke-linejoin="round"/>
  </g>
`;

const status = ({ x, color, label, mark }) => `
  <g transform="translate(${x} 0)">
    ${mark}
    <text x="38" y="7" class="mono status" fill="${cream}">${label}</text>
  </g>
`;

const svg = `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}">
  <defs>
    <pattern id="grid" width="108" height="104" patternUnits="userSpaceOnUse">
      <path d="M108 0H0V104" fill="none" stroke="${grid}" stroke-width="1"/>
    </pattern>
    <radialGradient id="glow" cx="50%" cy="36%" r="62%">
      <stop offset="0%" stop-color="#303331" stop-opacity="0.64"/>
      <stop offset="45%" stop-color="#202321" stop-opacity="0.36"/>
      <stop offset="100%" stop-color="#121514" stop-opacity="1"/>
    </radialGradient>
    <filter id="softShadow" x="-20%" y="-20%" width="140%" height="140%">
      <feDropShadow dx="0" dy="12" stdDeviation="11" flood-color="#000000" flood-opacity="0.42"/>
    </filter>
    <filter id="redGlow" x="-80%" y="-80%" width="260%" height="260%">
      <feGaussianBlur stdDeviation="5" result="blur"/>
      <feMerge>
        <feMergeNode in="blur"/>
        <feMergeNode in="SourceGraphic"/>
      </feMerge>
    </filter>
    <linearGradient id="creamFill" x1="0" x2="0" y1="0" y2="1">
      <stop offset="0%" stop-color="#f2ece1"/>
      <stop offset="100%" stop-color="#cec6ba"/>
    </linearGradient>
    <style>
      .display { font-family: "Avenir Next Condensed", "Avenir Next", "Helvetica Neue", Arial, sans-serif; font-weight: 900; letter-spacing: 0; }
      .sans { font-family: "Avenir Next", "Helvetica Neue", Arial, sans-serif; font-weight: 700; letter-spacing: 0; }
      .body { font-family: "Avenir Next", "Helvetica Neue", Arial, sans-serif; font-weight: 600; letter-spacing: 0; }
      .mono { font-family: Menlo, Consolas, monospace; letter-spacing: 0; }
      .brand { font-size: 36px; font-weight: 900; }
      .topline { font-size: 18px; fill: ${muted}; }
      .code { font-size: 18px; fill: ${red}; }
      .hero { font-size: 265px; filter: url(#softShadow); }
      .side { font-size: 35px; fill: ${cream}; font-weight: 800; }
      .side-small { font-size: 28px; fill: ${muted}; font-weight: 650; }
      .stat-label { font-size: 17px; fill: ${cream}; }
      .stat-index { font-size: 17px; fill: #97918a; }
      .stat-value { font-family: "Avenir Next Condensed", "Avenir Next", "Helvetica Neue", Arial, sans-serif; font-size: 74px; font-weight: 900; fill: url(#creamFill); filter: url(#softShadow); }
      .stat-body { font-family: "Avenir Next", "Helvetica Neue", Arial, sans-serif; font-size: 20px; font-weight: 600; fill: ${cream}; }
      .status { font-size: 15px; }
    </style>
  </defs>

  <rect width="${width}" height="${height}" fill="${panel}"/>
  <rect width="${width}" height="${height}" fill="url(#glow)"/>
  <rect width="${width}" height="${height}" fill="url(#grid)" opacity="0.82"/>

  <g opacity="0.35">
    <line x1="43" y1="573" x2="1529" y2="573" stroke="#5b5c58" stroke-width="1"/>
    <line x1="43" y1="882" x2="1529" y2="882" stroke="#565752" stroke-width="1"/>
  </g>

  <g transform="translate(67 77)">
    <image href="data:image/png;base64,${logoData}" x="0" y="-7" width="66" height="66"/>
    <text x="78" y="40" class="sans brand" fill="${cream}">AUTOMIC</text>
    <text x="265" y="40" class="sans brand" fill="${red}">VAULT</text>
    <line x1="421" y1="0" x2="421" y2="40" stroke="#86837b" stroke-width="1.4"/>
    <text x="456" y="32" class="mono topline">LOCAL SECURITY FOR AI AGENTS</text>
  </g>
  <text x="1435" y="120" text-anchor="middle" class="mono code">AV-2030</text>
  <line x1="43" y1="162" x2="1528" y2="162" stroke="${red}" stroke-width="2"/>

  <g transform="translate(66 277)">
    <text x="0" y="216" class="display hero" fill="url(#creamFill)">VAULT</text>
  </g>

  <g transform="translate(865 337)" filter="url(#redGlow)">
    <line x1="0" y1="52" x2="122" y2="52" stroke="${red}" stroke-width="14"/>
    <path d="M78 0 L132 52 L78 104" fill="none" stroke="${red}" stroke-width="14" stroke-linecap="square" stroke-linejoin="miter"/>
  </g>
  <g transform="translate(1067 330)">
    <text x="0" y="28" class="body side">Secrets stay sealed.</text>
    <text x="0" y="78" class="body side">Tools wait for approval.</text>
    <text x="0" y="126" class="body side-small">Local guardrails for agent-run CLIs.</text>
  </g>

  <g transform="translate(74 623)">
    ${stat({
      x: 0,
      index: "01",
      label: "ISOTOPES",
      value: "100+",
      body: "Package risk profiles",
      color: green,
      icon: `<path d="M25 -12 L36 -6 L36 7 L25 13 L14 7 L14 -6 Z" fill="none" stroke="${green}" stroke-width="2"/><path d="M25 -12 V13 M14 -6 L36 7 M36 -6 L14 7" stroke="${green}" stroke-width="1.6"/>`,
    })}
    <line x1="321" y1="-24" x2="321" y2="224" stroke="#62615c" stroke-width="1"/>
    ${stat({
      x: 363,
      index: "02",
      label: "APPROVALS",
      value: "GATED",
      body: "Gate risky actions",
      color: red,
      icon: `<circle cx="25" cy="-8" r="7" fill="none" stroke="${red}" stroke-width="2"/><path d="M12 13 C15 2 35 2 38 13 Z" fill="none" stroke="${red}" stroke-width="2"/>`,
    })}
    <line x1="693" y1="-24" x2="693" y2="224" stroke="#62615c" stroke-width="1"/>
    ${stat({
      x: 735,
      index: "03",
      label: "SECRETS",
      value: "SEALED",
      body: "No plaintext target",
      color: orange,
      icon: `<rect x="14" y="-1" width="22" height="17" rx="2" fill="none" stroke="${orange}" stroke-width="2"/><path d="M18 -1 V-8 C18 -18 32 -18 32 -8 V-1" fill="none" stroke="${orange}" stroke-width="2"/>`,
    })}
    <line x1="1066" y1="-24" x2="1066" y2="224" stroke="#62615c" stroke-width="1"/>
    ${stat({
      x: 1108,
      index: "04",
      label: "TOOL ROOTS",
      value: "LOCKED",
      body: "Agents cannot rewrite installs",
      color: red,
      icon: `<path d="M13 -7 H37 V13 H13 Z" fill="none" stroke="${red}" stroke-width="2"/><path d="M20 13 L16 20 M30 13 L34 20 M18 20 H32" stroke="${red}" stroke-width="2"/>`,
    })}
  </g>

  <g transform="translate(76 919)">
    ${status({
      x: 0,
      color: green,
      label: "ISOTOPE VERIFIED",
      mark: `<circle cx="15" cy="0" r="15" fill="none" stroke="${green}" stroke-width="2"/><path d="M7 0 L13 7 L24 -8" fill="none" stroke="${green}" stroke-width="2.2"/>`,
    })}
    <line x1="284" y1="-16" x2="284" y2="16" stroke="#5f5e59"/>
    ${status({
      x: 334,
      color: red,
      label: "ROOT PROTECTED",
      mark: `<path d="M15 -17 L28 -9 V9 L15 17 L2 9 V-9 Z" fill="none" stroke="${red}" stroke-width="2"/><path d="M10 3 V-4 C10 -11 20 -11 20 -4 V3" fill="none" stroke="${red}" stroke-width="1.8"/><rect x="8" y="3" width="14" height="10" rx="2" fill="none" stroke="${red}" stroke-width="1.8"/>`,
    })}
    <line x1="526" y1="-16" x2="526" y2="16" stroke="#5f5e59"/>
    ${status({
      x: 576,
      color: orange,
      label: "HUMAN AUTHORIZED",
      mark: `<path d="M15 -17 L28 -9 V9 L15 17 L2 9 V-9 Z" fill="none" stroke="${orange}" stroke-width="2"/><circle cx="15" cy="-4" r="4" fill="none" stroke="${orange}" stroke-width="1.8"/><path d="M8 10 C10 2 20 2 22 10" fill="none" stroke="${orange}" stroke-width="1.8"/>`,
    })}
    <line x1="814" y1="-16" x2="814" y2="16" stroke="#5f5e59"/>
    ${status({
      x: 864,
      color: green,
      label: "AUDIT LOG ENABLED",
      mark: `<circle cx="15" cy="0" r="15" fill="none" stroke="${green}" stroke-width="2"/><path d="M7 0 L13 7 L24 -8" fill="none" stroke="${green}" stroke-width="2.2"/>`,
    })}
    <line x1="1105" y1="-16" x2="1105" y2="16" stroke="#5f5e59"/>
    <g transform="translate(1150 0)">
      <circle cx="15" cy="0" r="15" fill="none" stroke="#88857d" stroke-width="2"/>
      <path d="M15 -9 V1 L22 6" fill="none" stroke="#88857d" stroke-width="2" stroke-linecap="round"/>
      <text x="45" y="7" class="mono status" fill="${cream}">UTC 2026-05-23 14:32:18</text>
    </g>
  </g>
</svg>
`;

mkdirSync(outDir, { recursive: true });
writeFileSync(tempSvgPath, svg);
execFileSync("magick", [tempSvgPath, "-strip", pngPath], { stdio: "inherit" });
execFileSync("magick", [pngPath, "-quality", "94", "-sampling-factor", "4:4:4", "-strip", jpgPath], { stdio: "inherit" });
rmSync(tempSvgPath, { force: true });
console.log(`Wrote ${pngPath}`);
console.log(`Wrote ${jpgPath}`);
