#!/usr/bin/env node

import { readdir, readFile, stat, writeFile } from "node:fs/promises";
import path from "node:path";

const [siteDirArg, outputArg] = process.argv.slice(2);

if (!siteDirArg || !outputArg) {
  console.error("Usage: generate-llms-full.mjs <site-dir> <output-file>");
  process.exit(1);
}

const siteDir = path.resolve(siteDirArg);
const outputFile = path.resolve(outputArg);
const includeExtensions = new Set([".html", ".md", ".txt"]);
const skippedNames = new Set(["llms-full.txt", ".DS_Store"]);
const skippedDirectories = new Set(["assets", "pagefind", "pkg"]);

async function walk(dir) {
  const entries = await readdir(dir, { withFileTypes: true });
  const files = [];

  for (const entry of entries) {
    const fullPath = path.join(dir, entry.name);
    if (entry.isDirectory()) {
      if (skippedDirectories.has(entry.name)) {
        continue;
      }
      files.push(...await walk(fullPath));
      continue;
    }
    if (!entry.isFile() || skippedNames.has(entry.name)) {
      continue;
    }
    if (includeExtensions.has(path.extname(entry.name))) {
      files.push(fullPath);
    }
  }

  return files;
}

function pagePath(file) {
  const relative = path.relative(siteDir, file).split(path.sep).join("/");
  if (relative === "index.html") {
    return "/";
  }
  if (relative.endsWith("/index.html")) {
    return `/${relative.slice(0, -"index.html".length)}`;
  }
  return `/${relative}`;
}

function decodeEntities(text) {
  return text
    .replace(/&amp;/g, "&")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&#39;/g, "'")
    .replace(/&nbsp;/g, " ");
}

function htmlToText(html) {
  return decodeEntities(html)
    .replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, " ")
    .replace(/<style\b[^>]*>[\s\S]*?<\/style>/gi, " ")
    .replace(/<svg\b[^>]*>[\s\S]*?<\/svg>/gi, " ")
    .replace(/<br\s*\/?>/gi, "\n")
    .replace(/<\/(p|div|section|article|header|footer|main|nav|h[1-6]|li|tr)>/gi, "\n")
    .replace(/<[^>]+>/g, " ")
    .replace(/[ \t]+\n/g, "\n")
    .replace(/\n{3,}/g, "\n\n")
    .replace(/[ \t]{2,}/g, " ")
    .trim();
}

function markdownToText(markdown) {
  return markdown.replace(/\n{3,}/g, "\n\n").trim();
}

function sortWeight(file) {
  const route = pagePath(file);
  if (route === "/") {
    return "0000";
  }
  if (route === "/docs/") {
    return "0001";
  }
  if (route === "/llms.txt") {
    return "0002";
  }
  return `1000${route}`;
}

const files = (await walk(siteDir)).sort((a, b) => sortWeight(a).localeCompare(sortWeight(b)));
const sections = [];

for (const file of files) {
  const info = await stat(file);
  if (info.size === 0) {
    continue;
  }
  const raw = await readFile(file, "utf8");
  const text = path.extname(file) === ".html" ? htmlToText(raw) : markdownToText(raw);
  if (!text) {
    continue;
  }
  sections.push(`# ${pagePath(file)}\n\n${text}`);
}

const output = [
  "# Automic Vault Full Site Text",
  "",
  "This file concatenates crawlable text from automicvault.com for AI systems and retrieval pipelines.",
  "",
  ...sections
].join("\n\n");

await writeFile(outputFile, `${output}\n`, "utf8");
