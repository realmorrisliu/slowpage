//! Sentence, link, and find coordinates for one chapter.
//!
//! Inside the typesetter the offset is a Unicode scalar into the parsed
//! chapter text, in document order. It does not move when the window reflows.
//! The history file and the WKWebView reader store a different number: the
//! UTF-16 length of `body` text, including whitespace this parser drops.
//! `to_web` and `from_web` translate at that boundary. A blocked page turn
//! reports offset 0 so the stored sentence stays.

use crate::parse::{Block, Chunk};

pub const SLIDE_MS: u32 = 160;

/// The nudge at a blocked chapter end. The page script uses the same 160 ms
/// and the same cap: 24 px, and at most 6 percent of the column.
pub fn blocked_ease(elapsed_ms: u32, width: f64, forward: bool) -> f64 {
    let amp = (width.abs() * 0.06).min(24.0);
    if !amp.is_finite() || amp <= 0.0 || elapsed_ms >= SLIDE_MS {
        return 0.0;
    }
    let u = f64::from(elapsed_ms) / f64::from(SLIDE_MS);
    let travel = (std::f64::consts::PI * u).sin() * amp;
    if forward {
        -travel
    } else {
        travel
    }
}

/// A blocked turn reports 0. A landed turn reports the sentence on screen.
pub fn turn_sentence(blocked: bool, sentence: u32) -> u32 {
    if blocked {
        0
    } else {
        sentence
    }
}

#[derive(Clone, Debug)]
struct NodeSpan {
    node: u32,
    start: u32,
    chars: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkSpan {
    pub start: u32,
    pub end: u32,
    pub href: String,
}

#[derive(Clone, Debug)]
pub struct ChapterText {
    plain: String,
    nodes: Vec<NodeSpan>,
    anchors: Vec<(String, u32)>,
    links: Vec<LinkSpan>,
    /// UTF-16 index, in the WKWebView body stream, of each kept scalar.
    at: Vec<u32>,
    web_units: u32,
}

pub fn index_blocks(blocks: &[Block], trailing: &[String], web_units: u32) -> ChapterText {
    let mut plain = String::new();
    let mut nodes = Vec::new();
    let mut anchors = Vec::new();
    let mut links: Vec<LinkSpan> = Vec::new();
    let mut at = Vec::new();
    let mut chars = 0u32;
    for block in blocks {
        for chunk in &block.chunks {
            let Chunk::Text(piece) = chunk else {
                continue;
            };
            let count = piece.text.chars().count() as u32;
            let start = chars;
            if count > 0 {
                nodes.push(NodeSpan {
                    node: piece.node,
                    start,
                    chars: count,
                });
            }
            for id in &piece.anchors {
                if !anchors.iter().any(|(name, _)| name == id) {
                    anchors.push((id.clone(), start));
                }
            }
            if count > 0 {
                if let Some(href) = &piece.link {
                    if let Some(open) = links.last_mut() {
                        if open.href == *href && open.end == start {
                            open.end = start + count;
                        } else {
                            links.push(LinkSpan {
                                start,
                                end: start + count,
                                href: href.clone(),
                            });
                        }
                    } else {
                        links.push(LinkSpan {
                            start,
                            end: start + count,
                            href: href.clone(),
                        });
                    }
                }
                let mut web = piece.web_at;
                for ch in piece.text.chars() {
                    at.push(web);
                    web = web.saturating_add(ch.len_utf16() as u32);
                }
                plain.push_str(&piece.text);
                chars += count;
            }
        }
    }
    for id in trailing {
        if !anchors.iter().any(|(name, _)| name == id) {
            anchors.push((id.clone(), chars));
        }
    }
    ChapterText {
        plain,
        nodes,
        anchors,
        links,
        at,
        web_units,
    }
}

pub fn chapter_text(xhtml: &str) -> ChapterText {
    let parsed = crate::parse::parse_chapter(xhtml, "", crate::parse::cjk_book(xhtml), None);
    index_blocks(&parsed.blocks, &parsed.trailing_anchors, parsed.web_units)
}

pub fn chapter_plain(xhtml: &str) -> String {
    chapter_text(xhtml).plain
}

impl ChapterText {
    pub fn plain(&self) -> &str {
        &self.plain
    }

    pub fn locate(&self, node: u32, char_offset: u32) -> u32 {
        let Some(span) = self.nodes.iter().find(|span| span.node == node) else {
            return 0;
        };
        span.start + char_offset.min(span.chars)
    }

    pub fn caret_at(&self, sentence: u32) -> Option<(u32, u32)> {
        if self.nodes.is_empty() {
            return None;
        }
        for (index, span) in self.nodes.iter().enumerate() {
            let end = span.start + span.chars;
            let last = index + 1 == self.nodes.len();
            if sentence < end || (last && sentence >= span.start) {
                return Some((
                    span.node,
                    sentence.saturating_sub(span.start).min(span.chars),
                ));
            }
        }
        None
    }

    pub fn slice(&self, start: u32, end: u32) -> String {
        if end <= start {
            return String::new();
        }
        self.plain
            .chars()
            .skip(start as usize)
            .take((end - start) as usize)
            .collect()
    }

    pub fn anchor(&self, id: &str) -> Option<u32> {
        self.anchors
            .iter()
            .find(|(name, _)| name == id)
            .map(|(_, at)| *at)
    }

    pub fn link_at(&self, sentence: u32) -> Option<&str> {
        self.links
            .iter()
            .find(|link| sentence >= link.start && sentence < link.end)
            .map(|link| link.href.as_str())
    }

    /// The WKWebView body offset for a typesetter sentence.
    pub fn to_web(&self, sentence: u32) -> u32 {
        let index = sentence as usize;
        if index >= self.at.len() {
            return self.web_units;
        }
        self.at[index]
    }

    /// The typesetter sentence at or before a WKWebView body offset.
    pub fn from_web(&self, utf16: u32) -> u32 {
        let count = self.at.len();
        if count == 0 || utf16 >= self.web_units {
            return count as u32;
        }
        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.at[mid] <= utf16 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo.saturating_sub(1) as u32
    }

    /// The first kept scalar at or after a WKWebView offset.
    ///
    /// A note's end is exclusive. `from_web` would stop on the scalar before
    /// a dropped newline and drop the last kept character of the note.
    pub fn from_web_end(&self, utf16: u32) -> u32 {
        let count = self.at.len();
        if count == 0 || utf16 >= self.web_units {
            return count as u32;
        }
        let mut lo = 0usize;
        let mut hi = count;
        while lo < hi {
            let mid = (lo + hi) / 2;
            if self.at[mid] < utf16 {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        lo as u32
    }

    /// A note range in WKWebView UTF-16 units. `end` is exclusive.
    pub fn to_web_span(&self, start: u32, end: u32) -> (u32, u32) {
        let (start, end) = ordered(start, end);
        (self.to_web(start), self.to_web(end))
    }

    /// A stored note range back in typesetter scalars. A zero-width web
    /// range stays zero-width. A web end that falls in dropped whitespace
    /// still includes the last kept scalar before that gap.
    pub fn from_web_span(&self, start: u32, end: u32) -> (u32, u32) {
        let (start, end) = ordered(start, end);
        if start == end {
            let at = self.from_web(start);
            return (at, at);
        }
        (self.from_web(start), self.from_web_end(end))
    }
}

fn ordered(start: u32, end: u32) -> (u32, u32) {
    if end < start {
        (end, start)
    } else {
        (start, end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_blocked_ease_returns_and_stays_inside_the_cap() {
        assert_eq!(blocked_ease(0, 400.0, true), 0.0);
        assert_eq!(blocked_ease(160, 400.0, true), 0.0);
        assert_eq!(blocked_ease(200, 400.0, false), 0.0);
        let mid = blocked_ease(80, 400.0, true);
        assert!((mid + 24.0).abs() < 1e-6);
        let narrow = blocked_ease(80, 100.0, false);
        assert!((narrow - 6.0).abs() < 1e-6);
        assert_eq!(turn_sentence(true, 40), 0);
        assert_eq!(turn_sentence(false, 40), 40);
    }

    #[test]
    fn links_and_anchors_share_the_chapter_text() {
        let xhtml =
            r#"<p>See <a href="next.xhtml#there">this</a> now.</p><h2 id="there">There</h2>"#;
        let parsed = crate::parse::parse_chapter(xhtml, "", false, None);
        let text = index_blocks(&parsed.blocks, &parsed.trailing_anchors, parsed.web_units);
        let plain = text.plain();
        let this_at = plain.find("this").unwrap() as u32;
        assert_eq!(text.link_at(this_at), Some("next.xhtml#there"));
        assert!(text.link_at(0).is_none());
        let there = text.anchor("there").unwrap();
        assert_eq!(text.slice(there, there + 5), "There");
        let (node, offset) = text.caret_at(this_at).unwrap();
        assert_eq!(text.locate(node, offset), this_at);
        assert_eq!(chapter_plain(xhtml), plain);
    }

    #[test]
    fn an_empty_anchor_points_at_the_following_text() {
        let text = chapter_text(r#"<a id="note"></a>Second"#);
        let at = text.anchor("note").expect("empty target");
        assert_eq!(text.slice(at, at + 6), "Second");

        let split = chapter_text("<p>First</p><a id=\"note\"></a><p>Second</p>");
        let at = split.anchor("note").expect("target between paragraphs");
        assert_eq!(split.slice(at, at + 6), "Second");

        let end = chapter_text("<p>First</p><a id=\"note\"></a>");
        assert_eq!(end.anchor("note"), Some(end.plain().chars().count() as u32));
    }

    #[test]
    fn a_web_offset_round_trips_onto_the_kept_sentence() {
        let text = chapter_text("<body>\n<p>First</p>\n<p>Second</p>\n</body>");
        assert_eq!(text.plain(), "FirstSecond");
        let second = text.plain().find("Second").unwrap() as u32;
        assert_eq!(text.to_web(second), 7);
        assert_eq!(text.from_web(7), second);
        assert_eq!(text.from_web(text.to_web(0)), 0);
        let end = text.plain().chars().count() as u32;
        for index in 0..=end {
            assert_eq!(text.from_web(text.to_web(index)), index);
        }

        let emoji = chapter_text("<p>A😀B</p>");
        assert_eq!(emoji.plain(), "A😀B");
        assert_eq!(emoji.to_web(0), 0);
        assert_eq!(emoji.to_web(1), 1);
        assert_eq!(emoji.to_web(2), 3);
        assert_eq!(emoji.from_web(2), 1);
        assert_eq!(emoji.from_web(3), 2);

        let hidden =
            chapter_text(r#"<body><div class="pg-boilerplate">Header</div><p>Body</p></body>"#);
        assert_eq!(hidden.plain(), "Body");
        assert_eq!(hidden.to_web(0), 6);
        assert_eq!(hidden.from_web(6), 0);

        let titled =
            chapter_text("<html><head><title>Title</title></head><body><p>Hi</p></body></html>");
        assert_eq!(titled.plain(), "Hi");
        assert_eq!(titled.to_web(0), 0);
    }

    #[test]
    fn a_named_footnote_lands_on_its_text() {
        let text = chapter_text(
            r##"<p>See <a href="#fn1" epub:type="noteref">1</a>.</p><aside epub:type="footnote"><p><a name="fn1"></a>The note.</p></aside>"##,
        );
        let mark = text.plain().find('1').unwrap() as u32;
        assert_eq!(text.link_at(mark), Some("#fn1"));
        let at = text.anchor("fn1").expect("name target");
        assert_eq!(text.slice(at, at + 8), "The note");

        let named = chapter_text(r#"<p><a name="fn1" id="fn1">There</a></p>"#);
        assert_eq!(
            named.slice(
                named.anchor("fn1").unwrap(),
                named.anchor("fn1").unwrap() + 5
            ),
            "There"
        );

        let both = chapter_text(r#"<p><a id="a" name="b"></a>Next</p>"#);
        assert_eq!(both.anchor("a"), both.anchor("b"));
        assert_eq!(
            both.slice(both.anchor("b").unwrap(), both.anchor("b").unwrap() + 4),
            "Next"
        );

        let hidden = chapter_text(
            r#"<div class="pg-boilerplate"><a name="fn1">Secret</a></div><p>Body</p>"#,
        );
        assert!(hidden.anchor("fn1").is_none());
        assert_eq!(hidden.plain(), "Body");
    }

    #[test]
    fn a_note_span_round_trips_across_dropped_whitespace() {
        let text = chapter_text("<body>\n<p>First</p>\n<p>Second</p>\n</body>");
        let second = text.plain().find("Second").unwrap() as u32;
        assert_eq!(text.to_web_span(0, second), (text.to_web(0), 7));
        assert_eq!(text.from_web_span(text.to_web(0), 7), (0, second));
        let (start, end) = text.from_web_span(0, 6);
        assert_eq!(text.slice(start, end), "First");
        assert_eq!(
            text.from_web_span(6, 6),
            (text.from_web(6), text.from_web(6))
        );
        assert_eq!(text.to_web_span(second, 0).1, 7);

        let emoji = chapter_text("<p>A😀B</p>");
        let (start, end) = emoji.from_web_span(1, 2);
        assert_eq!(emoji.slice(start, end), "😀");
        let (web_start, web_end) = emoji.to_web_span(1, 2);
        assert_eq!(emoji.from_web_span(web_start, web_end), (1, 2));
    }
}
