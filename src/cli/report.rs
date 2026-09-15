use std::io::{self, Write};

const DEFAULT_WIDTH: usize = 80;

#[derive(Clone, Copy)]
pub(crate) struct Style {
    pub(crate) color: bool,
    pub(crate) width: usize,
}

impl Style {
    pub(crate) fn plain() -> Self {
        Self {
            color: false,
            width: DEFAULT_WIDTH,
        }
    }

    pub(crate) fn terminal(color: bool, width: Option<usize>) -> Self {
        Self {
            color,
            width: width.unwrap_or(DEFAULT_WIDTH).clamp(8, DEFAULT_WIDTH),
        }
    }

    pub(crate) fn paint(self, code: &str, text: impl AsRef<str>) -> String {
        let text = text.as_ref();
        if self.color {
            format!("\x1b[{code}m{text}\x1b[0m")
        } else {
            text.to_string()
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum Tone {
    Plain,
    Heading,
    Accent,
    Success,
    Warning,
    Danger,
    Muted,
}

impl Tone {
    fn code(self) -> Option<&'static str> {
        match self {
            Self::Plain => None,
            Self::Heading => Some("1"),
            Self::Accent => Some("36"),
            Self::Success => Some("32"),
            Self::Warning => Some("33"),
            Self::Danger => Some("31"),
            Self::Muted => Some("2"),
        }
    }
}

/// Builds rail-based CLI reports while keeping every wrapped line inside its fence.
///
/// It also implements `Write` so existing hardeners get safe wrapping immediately;
/// new output should use `line` to state its continuation rail and semantic tone.
pub(crate) struct ReportBuilder<'a, W: Write + ?Sized> {
    output: &'a mut W,
    style: Style,
    pending: Vec<u8>,
}

impl<'a, W: Write + ?Sized> ReportBuilder<'a, W> {
    pub(crate) fn new(output: &'a mut W, style: Style) -> Self {
        Self {
            output,
            style,
            pending: Vec::new(),
        }
    }

    pub(crate) fn line(
        &mut self,
        first_prefix: &str,
        continuation_prefix: &str,
        text: impl AsRef<str>,
        tone: Tone,
    ) -> io::Result<()> {
        self.render(first_prefix, continuation_prefix, text.as_ref(), tone, true)
    }

    fn render_inferred(&mut self, line: &str, newline: bool) -> io::Result<()> {
        let (first, continuation, text) = split_rail(line);
        self.render(
            first,
            continuation,
            text,
            inferred_tone(first, text),
            newline,
        )
    }

    fn render(
        &mut self,
        first_prefix: &str,
        continuation_prefix: &str,
        text: &str,
        tone: Tone,
        newline: bool,
    ) -> io::Result<()> {
        let first_width = self
            .style
            .width
            .saturating_sub(display_width(first_prefix))
            .max(1);
        let continuation_width = self
            .style
            .width
            .saturating_sub(display_width(continuation_prefix))
            .max(1);
        let lines = wrap_text(text, first_width, continuation_width);
        for (index, line) in lines.iter().enumerate() {
            let prefix = if index == 0 {
                first_prefix
            } else {
                continuation_prefix
            };
            let prefix = prefix.to_string();
            let line = tone
                .code()
                .map_or_else(|| line.clone(), |code| self.style.paint(code, line));
            self.output.write_all(prefix.as_bytes())?;
            self.output.write_all(line.as_bytes())?;
            if newline || index + 1 < lines.len() {
                self.output.write_all(b"\n")?;
            }
        }
        Ok(())
    }
}

impl<W: Write + ?Sized> Write for ReportBuilder<'_, W> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.pending.extend_from_slice(bytes);
        while let Some(end) = self.pending.iter().position(|byte| *byte == b'\n') {
            let line = self.pending.drain(..=end).collect::<Vec<_>>();
            let line = std::str::from_utf8(&line[..line.len() - 1])
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            self.render_inferred(line, true)?;
        }
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        if !self.pending.is_empty() {
            let pending = std::mem::take(&mut self.pending);
            let line = std::str::from_utf8(&pending)
                .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))?;
            self.render_inferred(line, false)?;
        }
        self.output.flush()
    }
}

fn split_rail(line: &str) -> (&str, &str, &str) {
    for (first, continuation) in [
        ("│  ├─ ", "│  │  "),
        ("│  ╰─ ", "│     "),
        ("╭─ ", "│  "),
        ("├─ ", "│  "),
        ("└─ ", "│  "),
        ("╰─ ", "   "),
        ("◆ ", "│ "),
        ("◇ ", "│ "),
        ("│  • ", "│    "),
        ("│  ", "│  "),
        ("│ ", "│ "),
    ] {
        if let Some(text) = line.strip_prefix(first) {
            return (first, continuation, text);
        }
    }
    if line == "│" {
        ("│", "│", "")
    } else {
        ("", "", line)
    }
}

fn inferred_tone(prefix: &str, text: &str) -> Tone {
    if prefix == "╭─ " {
        Tone::Accent
    } else if prefix == "╰─ "
        && ["hardened", "installed", "migrated", "already"]
            .iter()
            .any(|word| text.to_ascii_lowercase().contains(word))
    {
        Tone::Success
    } else if prefix == "╰─ " && text.contains("cancelled") {
        Tone::Warning
    } else {
        Tone::Plain
    }
}

pub(super) fn wrap_text(text: &str, first_width: usize, continuation_width: usize) -> Vec<String> {
    if text.is_empty() {
        return vec![String::new()];
    }
    let mut lines = Vec::new();
    for paragraph in text.lines() {
        let mut width = if lines.is_empty() {
            first_width
        } else {
            continuation_width
        };
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let separator = usize::from(!line.is_empty());
            if display_width(&line) + separator + display_width(word) <= width {
                if separator == 1 {
                    line.push(' ');
                }
                line.push_str(word);
                continue;
            }
            if !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                width = continuation_width;
            }
            push_word(word, width, &mut line, &mut lines);
            width = continuation_width;
        }
        lines.push(line);
    }
    lines
}

fn push_word(word: &str, width: usize, line: &mut String, lines: &mut Vec<String>) {
    for ch in word.chars() {
        if display_width(line) + char_width(ch) > width && !line.is_empty() {
            lines.push(std::mem::take(line));
        }
        line.push(ch);
    }
}

fn display_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

// Conservatively count non-ASCII glyphs as double-width. That keeps CJK and emoji
// inside the rail without adding a terminal-width dependency to this security product.
fn char_width(ch: char) -> usize {
    usize::from(ch != '\u{200d}' && !('\u{300}'..='\u{36f}').contains(&ch))
        * if ch.is_ascii() { 1 } else { 2 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wraps_every_continuation_inside_the_rail() {
        for width in [40, 80, 120] {
            let mut output = Vec::new();
            let mut report = ReportBuilder::new(&mut output, Style::terminal(false, Some(width)));
            writeln!(
                report,
                "├─ verify installer identity, notarization, timestamp, package identity, signed native payload, Hardened Runtime, and safe extraction limits"
            )
            .unwrap();
            drop(report);

            let output = String::from_utf8(output).unwrap();
            assert!(
                output
                    .lines()
                    .all(|line| display_width(line) <= width.min(DEFAULT_WIDTH))
            );
            assert!(output.lines().skip(1).all(|line| line.starts_with("│  ")));
        }
    }

    #[test]
    fn conservatively_wraps_wide_characters() {
        let mut output = Vec::new();
        let mut report = ReportBuilder::new(&mut output, Style::terminal(false, Some(20)));
        writeln!(report, "├─ 路径路径路径路径路径路径路径路径路径路径").unwrap();
        drop(report);

        assert!(
            String::from_utf8(output)
                .unwrap()
                .lines()
                .all(|line| display_width(line) <= 20)
        );
    }

    #[test]
    fn color_is_semantic_and_optional() {
        let mut colored = Vec::new();
        let mut report = ReportBuilder::new(&mut colored, Style::terminal(true, Some(80)));
        writeln!(report, "╭─ doctor\n╰─ hardened example").unwrap();
        drop(report);
        let colored = String::from_utf8(colored).unwrap();
        assert!(colored.contains("╭─ \x1b[36mdoctor\x1b[0m"));
        assert!(colored.contains("\x1b[32mhardened example\x1b[0m"));

        let mut plain = Vec::new();
        let mut report = ReportBuilder::new(&mut plain, Style::plain());
        writeln!(report, "╭─ doctor\n╰─ hardened example").unwrap();
        drop(report);
        assert_eq!(
            String::from_utf8(plain).unwrap(),
            "╭─ doctor\n╰─ hardened example\n"
        );
    }
}
