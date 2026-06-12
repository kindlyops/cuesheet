//! Verbatim port of the vbs Typst cuesheet emitter (cmd/plt_cuesheet.go).
//!
//! Every literal below is copied from the Go implementation so the generated
//! cuesheet.typ matches the CLI output byte for byte for the same manifest.

use std::fmt::Write as _;

use crate::model::{Cue, EndAction, SheetManifest};

const CUE_NUMBER_COLOR: &str = "rgb(\"#235a68\")";

/// Go: renderCueSheet(manifest buildManifest) string
pub fn render_cuesheet(manifest: &SheetManifest) -> String {
    let mut b = String::new();

    let mut total = 0.0_f64;
    let mut max_dur = 0.0_f64;
    for c in &manifest.cues {
        total += c.duration_sec;
        if c.duration_sec > max_dur {
            max_dur = c.duration_sec;
        }
    }

    write_cuesheet_preamble(&mut b, manifest, total);

    b.push_str("#table(\n");
    b.push_str("  columns: (auto, auto, 1fr, 3cm, auto),\n");
    b.push_str("  stroke: none,\n");
    b.push_str("  inset: (x: 8pt, y: 9pt),\n");
    b.push_str(
        "  align: (left + horizon, center + horizon, left + horizon, left + horizon, center + horizon),\n",
    );
    b.push_str("  table.header(\n");
    b.push_str("    [], [],\n");
    b.push_str(
        "    text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[CUE], text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[DURATION], text(size: 7.5pt, fill: luma(45%), tracking: 0.5pt)[AFTER],\n",
    );
    b.push_str("  ),\n");
    b.push_str("  table.hline(stroke: 0.6pt + luma(55%)),\n");

    let mut elapsed = 0.0_f64;
    for c in &manifest.cues {
        elapsed += c.duration_sec;
        b.push_str(&cuesheet_row(c, elapsed, max_dur));
    }

    b.push_str(")\n");
    b
}

/// Go: writeCueSheetPreamble(b *strings.Builder, manifest buildManifest, total float64)
fn write_cuesheet_preamble(b: &mut String, manifest: &SheetManifest, total: f64) {
    b.push_str("#set page(paper: \"us-letter\", margin: (x: 1.5cm, top: 1.5cm, bottom: 1.7cm),\n");
    b.push_str("  footer: context [\n");
    b.push_str("    #set text(size: 7.5pt, fill: luma(55%))\n");
    b.push_str("    #line(length: 100%, stroke: 0.5pt + luma(78%))\n    #v(2pt)\n");
    b.push_str(
        "    #align(right)[#counter(page).display() / #counter(page).final().first()]\n  ])\n",
    );
    b.push_str(
        "#set text(font: (\"Helvetica Neue\", \"Arial\"), size: 10pt, number-width: \"tabular\")\n",
    );
    b.push_str("#show raw: set text(size: 7.5pt, fill: luma(50%))\n");
    b.push_str(
        "#let sparkbar(p) = box(width: 80%, height: 0.32em, fill: luma(90%))[#box(width: p * 1%, height: 100%, fill: rgb(\"#235a68\"))]\n\n",
    );

    b.push_str(
        "#grid(columns: (1fr, auto), align: (left + bottom, right + bottom), column-gutter: 12pt,\n",
    );
    let _ = writeln!(
        b,
        "  text(size: 18pt, weight: \"bold\")[{}],",
        escape_typst(&manifest.name)
    );
    let _ = writeln!(
        b,
        "  text(size: 9.5pt, fill: luma(40%))[{} ({}) · {} · {} cues · {}],",
        escape_typst(&manifest.language_code),
        manifest.language_id,
        manifest.resolution,
        manifest.cues.len(),
        format_timecode(total)
    );
    b.push_str(")\n#v(5pt)\n#line(length: 100%, stroke: 1pt)\n#v(6pt)\n\n");
}

/// Go: cueSheetRow(c cue, elapsed, maxDur float64) string
fn cuesheet_row(c: &Cue, elapsed: f64, max_dur: f64) -> String {
    let thumb = if c.thumbnail.is_empty() {
        "[]".to_string()
    } else {
        format!("[#image({}, width: 2cm)]", go_quote(&c.thumbnail))
    };
    let number = format!(
        "[#text(fill: {CUE_NUMBER_COLOR}, weight: \"bold\", size: 12pt)[{}]]",
        c.index
    );

    let pace = if max_dur > 0.0 {
        (c.duration_sec / max_dur).sqrt() * 100.0
    } else {
        0.0
    };
    let duration = format!(
        "[#stack(spacing: 3.5pt, [{}], sparkbar({:.1}), text(size: 7pt, fill: luma(62%))[elapsed {}])]",
        format_timecode(c.duration_sec),
        pace,
        format_timecode(elapsed)
    );

    format!(
        "  {}, {}, [#text(weight: 500)[{}] \\ #raw({})], {}, [#text(fill: luma(50%))[{}]],\n  table.hline(stroke: 0.3pt + luma(88%)),\n",
        number,
        thumb,
        escape_typst(&c.label),
        go_quote(&c.clip),
        duration,
        EndAction::from_code(c.end_action_raw).label()
    )
}

/// Go: formatTimecode(seconds float64) string — fmt.Sprintf("%d:%04.1f", ...)
pub fn format_timecode(seconds: f64) -> String {
    let s = if seconds < 0.0 { 0.0 } else { seconds };
    let minutes = (s as i64) / 60;
    let rem = s - (minutes * 60) as f64;
    format!("{minutes}:{rem:04.1}")
}

/// Go: escapeTypst(s string) string — simultaneous single-pass replacement.
/// Doing the backslash first then the rest sequentially is equivalent because
/// no later replacement introduces a character an earlier pass would touch.
pub fn escape_typst(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '#' => out.push_str("\\#"),
            '*' => out.push_str("\\*"),
            '_' => out.push_str("\\_"),
            '$' => out.push_str("\\$"),
            '[' => out.push_str("\\["),
            ']' => out.push_str("\\]"),
            '@' => out.push_str("\\@"),
            other => out.push(other),
        }
    }
    out
}

/// Equivalent of Go's %q for the strings that appear in this template:
/// double-quoted with backslash escapes for quote, backslash, and control
/// characters; printable characters pass through unchanged.
pub fn go_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\t' => out.push_str("\\t"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\x{:02x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn timecode_matches_go_format() {
        assert_eq!(format_timecode(0.0), "0:00.0");
        assert_eq!(format_timecode(5.25), "0:05.2");
        assert_eq!(format_timecode(65.0), "1:05.0");
        assert_eq!(format_timecode(-3.0), "0:00.0");
        assert_eq!(format_timecode(600.06), "10:00.1");
    }

    #[test]
    fn escape_typst_escapes_markup() {
        assert_eq!(
            escape_typst("a#b[c]*d_e$f@g\\h"),
            "a\\#b\\[c\\]\\*d\\_e\\$f\\@g\\\\h"
        );
        assert_eq!(escape_typst("plain"), "plain");
    }

    #[test]
    fn go_quote_quotes_like_percent_q() {
        assert_eq!(go_quote("a\"b"), "\"a\\\"b\"");
        assert_eq!(go_quote("tab\there"), "\"tab\\there\"");
        assert_eq!(go_quote("päth/file.mp4"), "\"päth/file.mp4\"");
    }

    #[test]
    fn end_action_labels_match_vbs() {
        assert_eq!(EndAction::from_code(0).label(), "continue");
        assert_eq!(EndAction::from_code(1).label(), "stop");
        assert_eq!(EndAction::from_code(2).label(), "freeze");
        assert_eq!(EndAction::from_code(7).label(), "code 7");
    }
}
