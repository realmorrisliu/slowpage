//! Sentence, link, and find coordinates for one chapter.
//!
//! The offset is a Unicode scalar into the parsed chapter text, in document
//! order. It does not move when the window reflows. A blocked page turn
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
}

pub fn index_blocks(blocks: &[Block]) -> ChapterText {
    let mut plain = String::new();
    let mut nodes = Vec::new();
    let mut anchors = Vec::new();
    let mut links: Vec<LinkSpan> = Vec::new();
    let mut chars = 0u32;
    for block in blocks {
        for chunk in &block.chunks {
            let Chunk::Text(piece) = chunk else {
                continue;
            };
            let count = piece.text.chars().count() as u32;
            let start = chars;
            nodes.push(NodeSpan {
                node: piece.node,
                start,
                chars: count,
            });
            for id in &piece.anchors {
                if !anchors.iter().any(|(name, _)| name == id) {
                    anchors.push((id.clone(), start));
                }
            }
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
            plain.push_str(&piece.text);
            chars += count;
        }
    }
    ChapterText {
        plain,
        nodes,
        anchors,
        links,
    }
}

pub fn chapter_plain(xhtml: &str) -> String {
    let blocks = crate::parse::parse_chapter(xhtml, "", crate::parse::cjk_book(xhtml), None);
    index_blocks(&blocks).plain
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
        let blocks = crate::parse::parse_chapter(xhtml, "", false, None);
        let text = index_blocks(&blocks);
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
}
