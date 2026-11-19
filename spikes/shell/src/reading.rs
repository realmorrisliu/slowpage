pub const FRAME_FLOOR_MS: f64 = 16.7;
pub const PAPER: &str = "#ffffff";

pub fn paper_u32() -> u32 {
    u32::from_str_radix(PAPER.trim_start_matches('#'), 16).expect("paper hex")
}

pub fn paper_rgb() -> (f64, f64, f64) {
    let value = paper_u32();
    (
        f64::from((value >> 16) & 0xff) / 255.0,
        f64::from((value >> 8) & 0xff) / 255.0,
        f64::from(value & 0xff) / 255.0,
    )
}
pub const SLIDE_MS: u16 = 160;
pub const TITLE_DELAY_MS: u32 = 400;
pub const TITLE_OVER_MS: u32 = 1;
pub const FONT_STACK: &str = "\"Iowan Old Style\", \"Songti SC\", serif";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Trim {
    pub id: &'static str,
    pub label: &'static str,
    pub width_mm: u16,
    pub height_mm: u16,
}

pub const TRIMS: [Trim; 3] = [
    Trim {
        id: "32kai",
        label: "32开",
        width_mm: 130,
        height_mm: 184,
    },
    Trim {
        id: "paperback",
        label: "Paperback",
        width_mm: 129,
        height_mm: 198,
    },
    Trim {
        id: "trade",
        label: "US Trade",
        width_mm: 140,
        height_mm: 216,
    },
];

pub const PAGE_OVERRIDES: &[(&str, &str)] = &[
    (
        "html and body margin, width, max-width, and height",
        "A publisher body width or margin shrinks the column box, so the chapter stays one page.",
    ),
    (
        "body columns, overflow, and page padding",
        "The page box, the measure, and the page margins belong to SuperReader.",
    ),
    (
        "direct children width, height, side margins, and float",
        "A wrapper with a fixed width or height becomes a single column.",
    ),
    (
        "position fixed rewritten to absolute",
        "A fixed element paints on every column.",
    ),
    (
        "a two pixel clip on the right edge",
        "The next column otherwise draws a dark line on the page edge.",
    ),
];

pub const DEFAULT_TRIM: &str = "paperback";

pub fn trim(id: &str) -> &'static Trim {
    TRIMS
        .iter()
        .find(|item| item.id == id)
        .unwrap_or_else(|| TRIMS.iter().find(|item| item.id == DEFAULT_TRIM).unwrap())
}

pub fn page_ratio(declared: Option<(f64, f64)>, item: &Trim) -> (f64, f64, &'static str) {
    if let Some((width, height)) = declared {
        if width > 0.0 && height > 0.0 {
            return (width, height, "declared");
        }
    }
    (f64::from(item.width_mm), f64::from(item.height_mm), "trim")
}

pub fn fit_page(window_w: f64, window_h: f64, ratio_w: f64, ratio_h: f64) -> (f64, f64, f64, f64) {
    let window_w = window_w.max(1.0);
    let window_h = window_h.max(1.0);
    let ratio_w = ratio_w.max(1.0);
    let ratio_h = ratio_h.max(1.0);
    let page = ratio_w / ratio_h;
    let (width, height) = if window_w / window_h > page {
        (window_h * page, window_h)
    } else {
        (window_w, window_w / page)
    };
    (
        ((window_w - width) / 2.0).max(0.0),
        ((window_h - height) / 2.0).max(0.0),
        width,
        height,
    )
}

pub fn ratio_within(width: f64, height: f64, want_w: f64, want_h: f64) -> bool {
    if height <= 0.0 || want_h <= 0.0 {
        return false;
    }
    let got = width / height;
    let want = want_w / want_h;
    (got - want).abs() / want <= 0.01
}

struct Face {
    size_px: u8,
    line_height: &'static str,
    measure_ch: u8,
    margin_top: u16,
    margin_side: u8,
    margin_bottom: u16,
    cjk: bool,
}

fn cjk_lang(lang: &str) -> bool {
    let lang = lang.trim().to_ascii_lowercase();
    lang.starts_with("zh") || lang.starts_with("ja")
}

fn face(lang: &str) -> Face {
    if cjk_lang(lang) {
        Face {
            size_px: 20,
            line_height: "1.70",
            measure_ch: 64,
            margin_top: 72,
            margin_side: 11,
            margin_bottom: 64,
            cjk: true,
        }
    } else {
        Face {
            size_px: 22,
            line_height: "1.48",
            measure_ch: 48,
            margin_top: 88,
            margin_side: 14,
            margin_bottom: 72,
            cjk: false,
        }
    }
}

pub fn clean_lang(lang: &str) -> String {
    let lang = lang.trim();
    if lang.is_empty() {
        return "en".to_string();
    }
    lang.chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect()
}

pub fn fallback_css(lang: &str) -> String {
    let face = face(lang);
    let cjk = if face.cjk {
        "text-autospace: normal; line-break: strict;"
    } else {
        ""
    };
    format!(
        r#"@layer spike-fallback {{
  :where(html, body) {{
    font-family: {stack};
    font-size: {size}px;
    line-height: {leading};
    text-align: justify;
    hyphens: auto;
    -webkit-hyphens: auto;
    -webkit-hyphenate-limit-before: 2;
    -webkit-hyphenate-limit-after: 3;
    -webkit-hyphenate-limit-lines: 2;
    hanging-punctuation: first allow-end last;
    {cjk}
  }}
  :where(p, li, blockquote) {{
    text-indent: 2em;
    margin-top: 0;
    margin-bottom: 0;
  }}
}}
"#,
        stack = FONT_STACK,
        size = face.size_px,
        leading = face.line_height,
        cjk = cjk,
    )
}

pub fn page_css(lang: &str) -> String {
    let face = face(lang);
    // The body is inset by the header and the footer, so those bands, and any
    // side gap the body does not cover, are the html canvas. That canvas and
    // the footer are paper. The body background is left alone, so a book's own
    // page color still wins inside the text block.
    format!(
        r#"
html, body {{
  color: #1c1915 !important;
  margin: 0 !important;
  width: auto !important;
  max-width: none !important;
  min-width: 0 !important;
  height: 100% !important;
}}
body {{
  position: absolute !important;
  left: 0 !important;
  right: 0 !important;
  top: {top}px !important;
  height: calc(100vh - {top}px - {bottom}px) !important;
  padding: 0 !important;
  box-sizing: border-box !important;
  margin-left: 0 !important;
  margin-right: 0 !important;
  column-gap: 0 !important;
  column-fill: auto !important;
  overflow: hidden !important;
}}
html {{
  background: {paper} !important;
  overflow: hidden !important;
}}
#spike-footer {{
  position: fixed !important;
  left: 0 !important;
  right: 0 !important;
  bottom: 0 !important;
  height: {bottom}px !important;
  pointer-events: none !important;
  background: {paper} !important;
  font-family: -apple-system, system-ui, sans-serif !important;
  font-size: 11px !important;
  font-weight: 400 !important;
  font-variant-numeric: tabular-nums !important;
  font-feature-settings: "tnum" 1 !important;
  color: rgba(60, 60, 67, 0.72) !important;
}}
#spike-footer .row {{
  display: flex !important;
  align-items: center !important;
  justify-content: space-between !important;
  height: calc(100% - 8px) !important;
  margin-left: max({side}%, calc((100% - {measure}ch) / 2)) !important;
  margin-right: max({side}%, calc((100% - {measure}ch) / 2)) !important;
}}
p, li, blockquote, h1, h2, h3 {{
  padding-left: max({side}%, calc((100% - {measure}ch) / 2)) !important;
  padding-right: max({side}%, calc((100% - {measure}ch) / 2)) !important;
  box-sizing: border-box !important;
}}
body > * {{
  width: auto !important;
  max-width: none !important;
  height: auto !important;
  min-height: 0 !important;
  margin-left: 0 !important;
  margin-right: 0 !important;
  float: none !important;
}}
body::-webkit-scrollbar {{ display: none; }}
"#,
        paper = PAPER,
        top = face.margin_top,
        bottom = face.margin_bottom,
        side = face.margin_side,
        measure = face.measure_ch,
    )
}

pub fn prepare_html(html: &str, lang: &str) -> String {
    let lang = clean_lang(lang);
    let mut html = html.to_string();
    set_lang(&mut html, &lang);
    let style = format!(
        "<style id=\"spike-fallback\">{}</style><style id=\"spike-paper\">html, body {{ background: {PAPER}; }}</style>",
        fallback_css(&lang)
    );
    let lower = html.to_ascii_lowercase();
    if let Some(at) = lower.find("<head>") {
        html.insert_str(at + "<head>".len(), &style);
    } else if let Some(at) = lower.find("<html") {
        if let Some(end) = html[at..].find('>') {
            html.insert_str(at + end + 1, &style);
        } else {
            html.insert_str(0, &style);
        }
    } else {
        html.insert_str(0, &style);
    }
    html
}

fn set_lang(html: &mut String, lang: &str) {
    let lower = html.to_ascii_lowercase();
    let Some(start) = lower.find("<html") else {
        return;
    };
    let Some(end_rel) = html[start..].find('>') else {
        return;
    };
    let end = start + end_rel;
    let tag = html[start..end].to_string();
    let tag_lower = tag.to_ascii_lowercase();
    if let Some(rel) = tag_lower.find("lang=") {
        let value_at = start + rel + "lang=".len();
        let bytes = html.as_bytes();
        if value_at >= bytes.len() {
            return;
        }
        let quote = bytes[value_at];
        if quote == b'"' || quote == b'\'' {
            let close = html[value_at + 1..]
                .find(quote as char)
                .map(|i| value_at + 1 + i);
            if let Some(close) = close {
                html.replace_range(value_at + 1..close, lang);
            }
        }
    } else {
        html.insert_str(start + "<html".len(), &format!(" lang=\"{lang}\""));
    }
}

pub fn footer_zh(page: u32, pages: u32) -> (String, String) {
    let pages = pages.max(1);
    let page = page.clamp(1, pages);
    let left = pages.saturating_sub(page);
    (format!("{page}/{pages}页"), format!("本章还剩{left}页"))
}

pub fn product_page_footer(page: u32, pages: u32) -> (String, String) {
    if pages == 0 {
        return (String::new(), String::new());
    }
    footer_zh(page, pages)
}

pub fn title_alpha(shown_ms: u32) -> f32 {
    fade_down(shown_ms, TITLE_DELAY_MS, TITLE_OVER_MS).clamp(0.0, 1.0)
}

fn fade_down(elapsed: u32, delay: u32, over: u32) -> f32 {
    let value = if elapsed <= delay {
        1.0
    } else if over == 0 {
        0.0
    } else {
        1.0 - (elapsed - delay) as f32 / over as f32
    };
    if value < 0.08 {
        0.0
    } else {
        value
    }
}

pub fn frames_within_floor(frames: &[f64]) -> bool {
    let missed = FRAME_FLOOR_MS * 2.0;
    !frames.is_empty() && frames.iter().all(|ms| ms.is_finite() && *ms < missed)
}

pub fn justify_source(enabled: bool) -> String {
    include_str!("../../justify/justify.js")
        .replace(
            "__TEX_LAYOUT__",
            include_str!("../../justify/vendor/tex-linebreak/layout.js"),
        )
        .replace(
            "__PATTERNS_US__",
            include_str!("../../justify/vendor/hyphenated-en-us/patterns.json"),
        )
        .replace(
            "__PATTERNS_GB__",
            include_str!("../../justify/vendor/hyphenated-en-gb/patterns.json"),
        )
        .replace("__JUSTIFY_ON__", if enabled { "true" } else { "false" })
        .replace("__BREAK_ALGO__", crate::breaks::BREAK_ALGO)
}

pub fn page_script(lang: &str) -> String {
    let lang = clean_lang(lang);
    let fallback =
        serde_json::to_string(&fallback_css(&lang)).unwrap_or_else(|_| "\"\"".to_string());
    let page = serde_json::to_string(&page_css(&lang)).unwrap_or_else(|_| "\"\"".to_string());
    let face = face(&lang);
    SCRIPT
        .replace("__LANG__", &lang)
        .replace("__FALLBACK__", &fallback)
        .replace("__PAGE__", &page)
        .replace("__MS__", &SLIDE_MS.to_string())
        .replace("__EDGE__", &crate::visit::PAGE_EDGE.to_string())
        .replace("__SLOP__", &crate::visit::CLICK_SLOP.to_string())
        .replace("__PAPER__", PAPER)
        .replace("__TOP__", &face.margin_top.to_string())
        .replace("__BOTTOM__", &face.margin_bottom.to_string())
}

const SCRIPT: &str = r#"
(function () {
  const fallback = document.createElement('style');
  fallback.id = 'spike-fallback-late';
  fallback.textContent = __FALLBACK__;
  (document.head || document.documentElement).prepend(fallback);
  const page = document.createElement('style');
  page.id = 'spike-page';
  page.textContent = __PAGE__;
  (document.head || document.documentElement).appendChild(page);
  document.documentElement.lang = "__LANG__";
  if (document.body) document.body.lang = "__LANG__";
  document.querySelectorAll('*').forEach(function (el) {
    if (el.closest && el.closest('#spike-footer')) return;
    if (getComputedStyle(el).position === 'fixed') {
      el.style.setProperty('position', 'absolute', 'important');
    }
  });
  (function keepEmDash() {
    const WJ = '\u2060';
    const nodes = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (!node.nodeValue || node.nodeValue.indexOf('\u2014') < 0) continue;
      if (node.parentElement && node.parentElement.closest('#spike-footer')) continue;
      nodes.push(node);
    }
    nodes.forEach(function (node) {
      const text = node.nodeValue;
      let next = '';
      for (let i = 0; i < text.length; i++) {
        const ch = text.charAt(i);
        if (ch === '\u2014' && i > 0 && text.charAt(i - 1) !== WJ) next += WJ;
        next += ch;
      }
      if (next !== text) node.nodeValue = next;
    });
  })();
  window.spikeOwnsBox = true;
  const DURATION = __MS__;
  const PAPER = "__PAPER__";
  const root = document.body;
  let busy = false;
  let queued = 0;
  let queuedToken = 0;
  let shift = 0;
  let followX = 0;
  let followGen = 0;
  let motion = 0;
  let committing = false;
  let measured = false;
  let layoutReport = null;
  let justifyReady = false;
  function seek(x) {
    shift = x;
    root.scrollLeft = x;
    document.documentElement.scrollLeft = x;
    root.style.transform = "none";
  }
  function pageWidth() {
    return Math.max(1, Math.round(window.innerWidth));
  }
  function columnStride() {
    return pageWidth();
  }
  const MARGIN_TOP = __TOP__;
  const MARGIN_BOTTOM = __BOTTOM__;
  function fitColumns() {
    const stride = columnStride();
    const content = Math.max(1, window.innerHeight - MARGIN_TOP - MARGIN_BOTTOM);
    root.style.setProperty('position', 'absolute', 'important');
    root.style.setProperty('top', MARGIN_TOP + 'px', 'important');
    root.style.setProperty('left', '0', 'important');
    root.style.setProperty('right', '0', 'important');
    root.style.setProperty('height', content + 'px', 'important');
    root.style.setProperty('padding', '0', 'important');
    root.style.setProperty('column-width', stride + 'px', 'important');
    root.style.setProperty('overflow', 'hidden', 'important');
    document.documentElement.style.setProperty('overflow', 'hidden', 'important');
    document.documentElement.style.setProperty('clip-path', 'inset(0)', 'important');
  }
  function reflowPage() {
    fitColumns();
    if (justifyReady && window.spikeJustify) window.spikeJustify.onPageBoxChanged();
  }
  function ensureFooter() {
    let bar = document.getElementById('spike-footer');
    if (!bar) {
      bar = document.createElement('div');
      bar.id = 'spike-footer';
      bar.innerHTML = '<div class="row"><span class="num"></span><span class="rest"></span></div>';
    }
    if (bar.parentNode !== document.documentElement) document.documentElement.appendChild(bar);
    bar.style.setProperty('position', 'fixed', 'important');
    return bar;
  }
  window.spikeFooter = function (num, rest) {
    const bar = ensureFooter();
    const number = bar.querySelector('.num');
    const remain = bar.querySelector('.rest');
    if (number) number.textContent = num || '';
    if (remain) remain.textContent = rest || '';
  };
  function alignFooter() {
    const bar = ensureFooter();
    const probe = document.querySelector('p, h1, h2');
    if (!probe) return;
    const pcs = getComputedStyle(probe);
    const row = bar.querySelector('.row');
    if (row) {
      row.style.setProperty('margin-left', pcs.paddingLeft, 'important');
      row.style.setProperty('margin-right', pcs.paddingRight, 'important');
    }
  }
  function columnFill() {
    alignFooter();
    const line = parseFloat(getComputedStyle(root).lineHeight) || 32;
    const box = root.getBoundingClientRect();
    let last = 0;
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    while (walker.nextNode()) {
      const node = walker.currentNode;
      if (!(node.textContent || '').trim()) continue;
      const range = document.createRange();
      range.selectNodeContents(node);
      const rects = range.getClientRects();
      for (let i = 0; i < rects.length; i++) {
        const r = rects[i];
        if (r.width < 1 || r.bottom < box.top || r.top > box.bottom) continue;
        if (r.bottom > last) last = r.bottom;
      }
    }
    const footer = document.getElementById('spike-footer');
    const row = footer ? footer.querySelector('.row') : null;
    const rowBox = row ? row.getBoundingClientRect() : null;
    const barStyle = footer ? getComputedStyle(footer) : null;
    const textTops = [];
    if (footer) {
      footer.querySelectorAll('.num, .rest').forEach(function (el) {
        const box = el.getBoundingClientRect();
        if (box.height > 0) textTops.push(box.top);
      });
    }
    const footerTextTop = textTops.length ? Math.min.apply(null, textTops) : null;
    const probe = document.querySelector('p, h1, h2') || root;
    const pcs = getComputedStyle(probe);
    const padLeft = parseFloat(pcs.paddingLeft) || 0;
    const padRight = parseFloat(pcs.paddingRight) || 0;
    return {
      fill_gap: box.bottom - last,
      line_h: line,
      content_bottom: box.bottom,
      last_bottom: last,
      footer_left: rowBox ? rowBox.left : null,
      footer_right: rowBox ? rowBox.right : null,
      footer_text_top: footerTextTop,
      footer_font: barStyle ? barStyle.fontFamily : "",
      footer_variant: barStyle ? barStyle.fontVariantNumeric : "",
      footer_size: barStyle ? barStyle.fontSize : "",
      measure_left: padLeft,
      measure_right: window.innerWidth - padRight
    };
  }
  fitColumns();
  window.addEventListener('resize', reflowPage);
  function column() {
    const w = columnStride();
    const pages = Math.max(1, Math.round(Math.max(root.scrollWidth, shift + w) / w));
    const page = Math.min(pages, Math.round(shift / w) + 1);
    return { page: page, pages: pages, w: w };
  }
  function marks() {
    const text = document.body.innerText || "";
    let tofu = 0;
    let gap = 0;
    for (let i = 0; i < text.length; i++) {
      const c = text.charCodeAt(i);
      if (c === 0xFFFD || c === 0x25A1 || (c >= 0xE000 && c <= 0xF8FF)) tofu++;
      if (i + 1 < text.length && cjk(c) && text[i + 1] === " ") {
        let j = i + 1;
        while (j < text.length && text[j] === " ") j++;
        if (j < text.length && cjk(text.charCodeAt(j))) gap++;
      }
    }
    return { tofu: tofu, cjk_gap: gap };
  }
  function cjk(c) {
    return (c >= 0x3400 && c <= 0x9FFF) || (c >= 0xF900 && c <= 0xFAFF) || (c >= 0x3001 && c <= 0x303F);
  }
  function sample() {
    const node = document.querySelector('p') || document.body;
    const cs = getComputedStyle(node);
    const body = getComputedStyle(document.body);
    let fontStatus = 'unknown';
    try {
      if (document.fonts && document.fonts.check('19px "Literata"')) fontStatus = 'loaded';
      else if (document.fonts) fontStatus = 'missing';
    } catch (err) {
      fontStatus = 'missing';
    }
    return {
      font: cs.fontFamily,
      size: cs.fontSize,
      leading: cs.lineHeight,
      indent: cs.textIndent,
      align: cs.textAlign,
      hanging: body.hangingPunctuation || body.webkitHangingPunctuation || "",
      autospace: body.textAutospace || "",
      fontStatus: fontStatus,
      background: body.backgroundColor
    };
  }
  function gapRatio() {
    const canvas = document.createElement('canvas');
    const ctx = canvas.getContext('2d');
    if (!ctx) return 0;
    ctx.font = getComputedStyle(document.body).font;
    const normal = ctx.measureText(' ').width || 1;
    let max = normal;
    const paras = document.querySelectorAll('p');
    const count = Math.min(paras.length, 6);
    for (let i = 0; i < count; i++) {
      const walker = document.createTreeWalker(paras[i], NodeFilter.SHOW_TEXT);
      while (walker.nextNode()) {
        const node = walker.currentNode;
        const text = node.textContent || '';
        let spaces = 0;
        for (let at = 0; at < text.length && spaces < 30; at++) {
          if (text[at] !== ' ' || at === 0 || at + 1 >= text.length) continue;
          if (text[at - 1] === ' ' || text[at + 1] === ' ') continue;
          const left = document.createRange();
          left.setStart(node, at - 1);
          left.setEnd(node, at);
          const right = document.createRange();
          right.setStart(node, at + 1);
          right.setEnd(node, at + 2);
          const a = left.getBoundingClientRect();
          const b = right.getBoundingClientRect();
          if (!a.width || !b.width || Math.abs(a.top - b.top) > 2) continue;
          if (a.top < 0 || a.bottom > window.innerHeight || a.left < 0 || b.right > window.innerWidth) continue;
          const gap = b.left - a.right;
          if (gap > max) max = gap;
          spaces++;
        }
      }
    }
    return max / normal;
  }
  function post(extra) {
    const box = column();
    const fill = columnFill();
    const body = { page: box.page, pages: box.pages, sx: root.scrollLeft, view_w: window.innerWidth, leaf: !!document.getElementById("spike-leaf"), sentence: heldOffset() };
    body.fill_gap = fill.fill_gap;
    body.line_h = fill.line_h;
    body.content_bottom = fill.content_bottom;
    body.last_bottom = fill.last_bottom;
    body.footer_text_top = fill.footer_text_top;
    body.footer_left = fill.footer_left;
    body.footer_right = fill.footer_right;
    body.footer_font = fill.footer_font;
    body.footer_variant = fill.footer_variant;
    body.footer_size = fill.footer_size;
    body.measure_left = fill.measure_left;
    body.measure_right = fill.measure_right;
    const mark = marks();
    body.tofu = mark.tofu;
    body.cjk_gap = mark.cjk_gap;
    if (layoutReport) {
      body.layout = layoutReport;
      body.gap_ratio = layoutReport.gap_ratio;
    }
    if (!measured && box.pages > 0) {
      measured = true;
      body.sample = sample();
      if (!layoutReport) body.gap_ratio = gapRatio();
      body.plain = (document.body.innerText || '').slice(0, 24000);
    }
    if (extra) {
      Object.keys(extra).forEach(function (key) { body[key] = extra[key]; });
    }
    if (!body.kind) body.kind = "box";
    body.bold_prefixes = [];
    var probeRoot = document.getElementById("sr-bold-probes");
    if (probeRoot) body.bold_prefixes = Array.from(probeRoot.querySelectorAll(".sr-word-start")).map(function (el) { return el.textContent; });
    window.webkit.messageHandlers.spike.postMessage(JSON.stringify(body));
  }
  function locate(offset) {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    let seen = 0;
    while (walker.nextNode()) {
      const node = walker.currentNode;
      const next = seen + node.textContent.length;
      if (next > offset) return { node: node, off: Math.max(0, offset - seen) };
      seen = next;
    }
    return null;
  }
  function offsetOf(node, off) {
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    let seen = 0;
    while (walker.nextNode()) {
      if (walker.currentNode === node) return seen + off;
      seen += walker.currentNode.textContent.length;
    }
    return seen;
  }
  let lastPaint = null;
  function unwrapMarks() {
    document.querySelectorAll("mark.spike").forEach(function (mark) {
      const parent = mark.parentNode;
      if (!parent) return;
      while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
      parent.removeChild(mark);
      parent.normalize();
    });
  }
  function textSlices(start, end) {
    const slices = [];
    const walker = document.createTreeWalker(document.body, NodeFilter.SHOW_TEXT);
    let seen = 0;
    while (walker.nextNode()) {
      const node = walker.currentNode;
      const next = seen + node.textContent.length;
      const from = Math.max(start, seen);
      const to = Math.min(end, next);
      if (from < to) slices.push({ node: node, start: from - seen, end: to - seen });
      if (next >= end) break;
      seen = next;
    }
    return slices;
  }
  window.spikeClearPaint = function () {
    const range = lastPaint;
    unwrapMarks();
    return range;
  };
  window.spikePaint = function (start, end) {
    lastPaint = { start: start, end: end };
    unwrapMarks();
    const slices = textSlices(start, end);
    if (!slices.length) return "miss";
    const written = [];
    try {
      slices.forEach(function (slice) {
        const range = document.createRange();
        range.setStart(slice.node, slice.start);
        range.setEnd(slice.node, slice.end);
        const mark = document.createElement("mark");
        mark.className = "spike";
        range.surroundContents(mark);
        written.push(mark);
      });
    } catch (err) {
      written.forEach(function (mark) {
        const parent = mark.parentNode;
        if (!parent) return;
        while (mark.firstChild) parent.insertBefore(mark.firstChild, mark);
        parent.removeChild(mark);
      });
      return "split";
    }
    return "ok";
  };
  function heldOffset() {
    const x = Math.max(24, Math.min(80, window.innerWidth * 0.2));
    const y = Math.max(48, Math.min(140, window.innerHeight * 0.18));
    const caret = document.caretRangeFromPoint(x, y) || document.caretRangeFromPoint(x, y + 40);
    if (!caret || !caret.startContainer) return 0;
    const node = caret.startContainer.nodeType === 3
      ? caret.startContainer
      : (caret.startContainer.firstChild || null);
    if (!node || node.nodeType !== 3) return 0;
    return offsetOf(node, caret.startOffset || 0);
  }
  window.spikeHold = function () {
    const offset = heldOffset();
    window.spikeHeld = offset;
    post({ hold: offset });
    return offset;
  };
  function endFollow() {
    followGen++;
    motion++;
    followX = 0;
    queued = 0;
    queuedToken = 0;
    busy = false;
    committing = false;
    root.style.transform = "none";
  }
  window.spikeReveal = function (offset) {
    endFollow();
    const seekOffset = function () {
    requestAnimationFrame(function () {
      requestAnimationFrame(function () {
        const found = locate(offset);
        if (!found) {
          post({ pinned: offset, missed: true });
          return;
        }
        const prev = root.style.transform;
        root.style.transform = 'none';
        root.scrollLeft = 0;
        const range = document.createRange();
        range.setStart(found.node, found.off);
        range.setEnd(found.node, Math.min(found.node.textContent.length, found.off + 1));
        const rect = range.getBoundingClientRect();
        const w = columnStride();
        const index = Math.max(0, Math.floor((rect.left + 8) / w));
        root.style.transform = prev;
        seek(index * w);
        measured = false;
        post({ pinned: offset });
      });
    });
    };
    if (window.spikeJustify) window.spikeJustify.whenVisibleCommitted().then(seekOffset);
    else seekOffset();
  };
  function settle(frames, token) {
    document.documentElement.style.transform = "none";
    const leaf = document.getElementById("spike-leaf");
    if (leaf) leaf.remove();
    post(token ? { frames: frames, swipe: token } : { frames: frames });
    busy = false;
    window.spikeBusy = false;
    const next = queued;
    const nextToken = queuedToken;
    queued = 0;
    queuedToken = 0;
    if (next) begin(next, nextToken);
  }
  window.spikeTurn = function (delta) {
    // A commit glide is the turn that was already decided. Bumping the
    // generation here kills the only callback that clears `busy` and runs
    // the queued key, so the page stops accepting turns. A return glide has
    // not committed, and a key cancels it. The key takes the one queue slot,
    // so a swipe token waiting in that slot is retired.
    if (!committing) {
      followGen++;
      followX = 0;
      root.style.transform = "none";
    }
    if (queuedToken) {
      var dropped = queuedToken;
      queuedToken = 0;
      post({ swipe: dropped, dropped: true });
    }
    delta = delta < 0 ? -1 : 1;
    if (busy) {
      queued = delta;
      return "queued";
    }
    begin(delta, 0);
    return "ok";
  };
  function begin(delta, token) {
    const box = column();
    const target = box.page + delta;
    if (target < 1 || target > box.pages) {
      post({ kind: "turn", edge: delta < 0 ? "prev" : "next", frames: [] });
      return;
    }
    window.spikeBusy = true;
    window.spikeCover = box.w * (target - 1);
    if (justifyReady && window.spikeJustify) window.spikeJustify.onPageBoxChanged();
    window.spikeCover = null;
    busy = true;
    slide(box, delta, token || 0);
  }
  function slide(box, delta, token) {
    seek(box.w * (box.page + delta - 1));
    const live = motion;
    const leaf = document.createElement("div");
    leaf.id = "spike-leaf";
    leaf.style.cssText = "position:fixed;inset:0;z-index:5;pointer-events:none;background:" + PAPER + ";will-change:transform";
    document.documentElement.appendChild(leaf);
    const frames = [];
    let last = 0;
    const t0 = performance.now();
    function frame(t) {
      if (live !== motion) return;
      if (last) frames.push(t - last);
      last = t;
      const u = Math.min(1, (t - t0) / DURATION);
      const eased = (1 - Math.cos(Math.PI * u)) / 2;
      leaf.style.transform = "translate3d(" + (delta * eased * box.w) + "px,0,0)";
      if (u < 1) {
        requestAnimationFrame(frame);
        return;
      }
      if (live !== motion) return;
      settle(frames, token || 0);
    }
    requestAnimationFrame(frame);
  }
  document.addEventListener("mouseup", function () {
    const sel = window.getSelection();
    if (!sel || sel.rangeCount === 0 || sel.isCollapsed) return;
    const range = sel.getRangeAt(0);
    if (!range.startContainer || range.startContainer.nodeType !== 3) return;
    if (!range.endContainer || range.endContainer.nodeType !== 3) return;
    const start = offsetOf(range.startContainer, range.startOffset || 0);
    const end = offsetOf(range.endContainer, range.endOffset || 0);
    const text = (document.body.textContent || "").slice(Math.min(start, end), Math.max(start, end));
    if (!text.trim()) return;
    window.webkit.messageHandlers.spike.postMessage(JSON.stringify({
      kind: "selection", start: start, end: end, text: text
    }));
  }, true);
  window.spikeSeekLast = function () {
    const w = pageWidth();
    const pages = Math.max(1, Math.round(Math.max(root.scrollWidth, w) / w));
    seek((pages - 1) * w);
    post({ kind: "seek" });
  };
  let edgeX = 0;
  let edgeY = 0;
  let edgeDrag = false;
  window.addEventListener("mousedown", function (event) {
    if (event.button !== 0) return;
    edgeX = event.clientX;
    edgeY = event.clientY;
    edgeDrag = false;
  }, true);
  window.addEventListener("mousemove", function (event) {
    if (event.buttons !== 1) return;
    if (Math.abs(event.clientX - edgeX) >= __SLOP__ || Math.abs(event.clientY - edgeY) >= __SLOP__) edgeDrag = true;
  }, true);
  window.addEventListener("click", function (event) {
    if (event.button !== 0) return;
    const dragged = edgeDrag;
    edgeDrag = false;
    if (dragged) return;
    const bar = document.getElementById('spike-footer');
    if (bar) {
      const box = bar.getBoundingClientRect();
      if (event.clientX >= box.left && event.clientX <= box.right && event.clientY >= box.top && event.clientY <= box.bottom) return;
    }
    const w = window.innerWidth;
    if (!(w > 0)) return;
    const band = w * __EDGE__;
    let delta = 0;
    if (event.clientX < band) delta = -1;
    else if (event.clientX > w - band) delta = 1;
    else return;
    event.preventDefault();
    window.spikeTurn(delta);
  }, true);
  window.addEventListener("keydown", function (event) {
    if (event.repeat) return;
    var delta = 0;
    if (event.key === "ArrowRight" || (event.key === " " && !event.shiftKey)) delta = 1;
    else if (event.key === "ArrowLeft" || (event.key === " " && event.shiftKey)) delta = -1;
    else return;
    event.preventDefault();
    window.spikeTurn(delta);
  }, true);
  window.spikeFollowDrop = function () {
    endFollow();
  };
  window.spikeFollow = function (offset) {
    if (busy) return;
    followGen++;
    const w = pageWidth();
    const box = column();
    let x = offset;
    if (box.page <= 1 && x < 0) x = 0;
    if (box.page >= box.pages && x > 0) x = 0;
    if (w > 0 && x > w) x = w;
    if (w > 0 && x < -w) x = -w;
    followX = x;
    root.style.transform = x ? "translate3d(" + (-x) + "px,0,0)" : "none";
  };
  function glide(from, to, done) {
    const gen = followGen;
    const t0 = performance.now();
    function frame(t) {
      if (gen !== followGen) return;
      const u = Math.min(1, (t - t0) / DURATION);
      const eased = (1 - Math.cos(Math.PI * u)) / 2;
      const x = from + (to - from) * eased;
      root.style.transform = "translate3d(" + (-x) + "px,0,0)";
      if (u < 1) {
        requestAnimationFrame(frame);
        return;
      }
      if (gen === followGen) done();
    }
    requestAnimationFrame(frame);
  }
  window.spikeFollowEnd = function (delta, token) {
    if (busy) {
      // A key slide is not a commit. Drop the follow paint and let that slide
      // finish. A commit glide is the turn already decided: cancelling its
      // frame leaves `busy` set. A second lift queues one page and keeps that
      // token for the queued page. A cancel does not.
      if (!committing) {
        followGen++;
        followX = 0;
        root.style.transform = "none";
        return "no";
      }
      if (delta) {
        queued = delta < 0 ? -1 : 1;
        queuedToken = token || 0;
        return "queued";
      }
      return "no";
    }
    const from = followX;
    const w = pageWidth();
    const box = column();
    const blocked = !delta || (delta > 0 && box.page >= box.pages) || (delta < 0 && box.page <= 1);
    if (blocked) {
      glide(from, 0, function () {
        followX = 0;
        root.style.transform = "none";
      });
      return "no";
    }
    const step = delta > 0 ? 1 : -1;
    committing = true;
    busy = true;
    glide(from, step > 0 ? w : -w, function () {
      committing = false;
      followX = 0;
      root.style.transform = "none";
      seek(box.w * (box.page + step - 1));
      post(token ? { swipe: token } : null);
      busy = false;
      const next = queued;
      const nextToken = queuedToken;
      queued = 0;
      queuedToken = 0;
      if (next) begin(next, nextToken);
    });
    return "glide";
  };
  if (window.spikeJustify) {
    window.spikeJustify.install({
      onReport: function (report) {
        layoutReport = report;
        post();
      }
    });
    justifyReady = true;
    window.spikeJustify.onPageBoxChanged();
  }
  if (document.fonts && document.fonts.ready) {
    document.fonts.ready.then(function () {
      measured = false;
      if (window.spikeJustify) window.spikeJustify.onPageBoxChanged();
      else post();
    });
  }
  requestAnimationFrame(function () { post(); });
})();
"#;

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn chinese_footer_matches_the_books_line() {
        let (left, right) = footer_zh(590, 1146);
        assert_eq!(left, "590/1146页");
        assert_eq!(right, "本章还剩556页");
        let (left, right) = product_page_footer(2, 5);
        assert_eq!(left, "2/5页");
        assert_eq!(right, "本章还剩3页");
        assert_eq!(product_page_footer(1, 0), (String::new(), String::new()));
        let (left, right) = footer_zh(2, 2);
        assert_eq!(left, "2/2页");
        assert_eq!(right, "本章还剩0页");
    }

    #[test]
    fn space_and_shift_space_use_the_existing_page_turn() {
        let script = page_script("zh");
        assert!(script.contains("event.key === \" \""));
        assert!(script.contains("event.shiftKey"));
        assert!(script.contains("window.spikeTurn(delta)"));
        assert!(script.contains("target > box.pages"));
        assert!(script.contains("window.spikeFollow"));
        assert!(script.contains("window.spikeFollowDrop"));
        assert!(script.contains("window.spikeFollowEnd"));
        assert!(script.contains("DURATION"));
        assert!(!script.contains("addEventListener(\"wheel\""));
    }

    #[test]
    fn a_key_during_the_release_still_turns() {
        let script = page_script("en");
        assert!(!script.contains("addEventListener(\"wheel\""));
        let dir = std::env::temp_dir();
        let page = dir.join("superreader-release-glide.js");
        let drive = dir.join("superreader-release-glide-drive.js");
        std::fs::write(&page, script).unwrap();
        std::fs::write(&drive, super::RELEASE_GLIDE_DRIVE).unwrap();
        let ran = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page)
            .output()
            .expect("node");
        let _ = std::fs::remove_file(&page);
        let _ = std::fs::remove_file(&drive);
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&ran.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&ran.stdout)));
        let queued = serde_json::json!(["queued"]);
        let turned = serde_json::json!(["ok"]);
        assert_eq!(v["key"]["space"], queued);
        assert_eq!(v["key"]["arrow"], queued);
        assert_eq!(v["key"]["edge"], queued);
        // 400 is the harness page width. The release seeks one page and the
        // single queued turn seeks the next, so the column is 800.
        assert_eq!(v["key"]["scroll"], 800);
        assert_eq!(v["key"]["after"], turned);
        assert_eq!(v["drop"]["droppedScroll"], 0);
        assert_eq!(v["drop"]["after"], turned);
        assert_eq!(v["drop"]["scroll"], 400);
        assert_eq!(v["cancel"]["during"], turned);
        assert_eq!(v["cancel"]["scroll"], 400);
        assert_eq!(v["again"]["scroll"], 800);
        assert_eq!(v["again"]["after"], turned);
        assert_eq!(v["ignored"]["scroll"], 400);
        assert_eq!(v["ignored"]["after"], turned);
    }

    #[test]
    fn a_reveal_during_the_release_does_not_post() {
        let script = page_script("en");
        let dir = std::env::temp_dir();
        let page = dir.join("superreader-release-reveal.js");
        let drive = dir.join("superreader-release-reveal-drive.js");
        std::fs::write(&page, script).unwrap();
        std::fs::write(&drive, super::REVEAL_DURING_RELEASE).unwrap();
        let ran = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page)
            .output()
            .expect("node");
        let _ = std::fs::remove_file(&page);
        let _ = std::fs::remove_file(&drive);
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&ran.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&ran.stdout)));
        assert_eq!(v["reveal"]["scroll"], 0);
        assert_eq!(v["reveal"]["unpinned"], serde_json::json!([]));
        assert_eq!(v["reveal"]["pinned"], 800);
        assert_eq!(v["drop"]["scroll"], 0);
        assert_eq!(v["drop"]["posts"], 0);
        assert_eq!(v["key"]["during"], serde_json::json!(["queued"]));
        assert_eq!(v["key"]["scroll"], 800);
        assert_eq!(v["key"]["after"], serde_json::json!(["ok"]));
    }

    #[test]
    fn a_commit_post_names_its_token_and_a_second_finger_key_does_not_drop_it() {
        let script = page_script("en");
        let dir = std::env::temp_dir();
        let page = dir.join("superreader-commit-token.js");
        let drive = dir.join("superreader-commit-token-drive.js");
        std::fs::write(&page, script).unwrap();
        std::fs::write(&drive, super::COMMIT_TOKEN_DRIVE).unwrap();
        let ran = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page)
            .output()
            .expect("node");
        let _ = std::fs::remove_file(&page);
        let _ = std::fs::remove_file(&drive);
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&ran.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&ran.stdout)));
        assert_eq!(v["space"]["during"], serde_json::json!(["queued"]));
        assert_eq!(v["space"]["scroll"], 800);
        assert_eq!(v["space"]["swipe"], 7);
        assert_eq!(v["arrow"]["during"], serde_json::json!(["queued"]));
        assert_eq!(v["arrow"]["scroll"], 800);
        assert_eq!(v["other"]["scroll"], 400);
        assert_eq!(v["other"]["swipe"], 7);
        assert_eq!(v["late"]["swipe"], 7);
        assert_eq!(v["late"]["page"], 2);
        assert_eq!(v["reveal"]["scroll"], 0);
        assert_eq!(v["reveal"]["unpinned"], serde_json::json!([]));
        assert_eq!(v["reveal"]["pinned"], 800);
    }

    #[test]
    fn a_second_commit_and_a_long_chapter_post_every_token() {
        let script = page_script("en");
        let dir = std::env::temp_dir();
        let page = dir.join("superreader-commit-chain.js");
        let drive = dir.join("superreader-commit-chain-drive.js");
        std::fs::write(&page, script).unwrap();
        std::fs::write(&drive, super::COMMIT_CHAIN_DRIVE).unwrap();
        let ran = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page)
            .output()
            .expect("node");
        let _ = std::fs::remove_file(&page);
        let _ = std::fs::remove_file(&drive);
        assert!(
            ran.status.success(),
            "{}",
            String::from_utf8_lossy(&ran.stderr)
        );
        let v: serde_json::Value = serde_json::from_slice(&ran.stdout)
            .unwrap_or_else(|_| panic!("{}", String::from_utf8_lossy(&ran.stdout)));
        let down = crate::visit::WheelTick {
            finger: crate::visit::Finger::Down,
            momentum: false,
            dx: 80.0,
            dy: 0.0,
        };
        let up = crate::visit::WheelTick {
            finger: crate::visit::Finger::Up,
            momentum: false,
            dx: 0.0,
            dy: 0.0,
        };
        let issue = |swipe: &mut crate::visit::Swipe| {
            assert_eq!(
                swipe.offer(down, 4, 400),
                crate::visit::WheelMove::Follow(80.0)
            );
            assert_eq!(swipe.offer(up, 4, 400), crate::visit::WheelMove::Commit(1));
            swipe.commit_token().expect("token")
        };

        assert_eq!(v["second"]["first"], "glide");
        assert_eq!(v["second"]["second"], "queued");
        assert_eq!(v["second"]["swipes"], serde_json::json!([1, 2]));
        let mut paired = crate::visit::Swipe::default();
        let first = issue(&mut paired);
        paired.note_answer(first, v["second"]["first"].as_str().unwrap());
        let second = issue(&mut paired);
        paired.note_answer(second, v["second"]["second"].as_str().unwrap());
        assert_eq!(
            crate::visit::swipe_progress(&mut paired, Some(second), 3, 0),
            Some((3, 0))
        );
        assert_eq!(
            crate::visit::swipe_progress(&mut paired, Some(first), 2, 0),
            Some((2, 0))
        );
        assert_eq!(
            crate::visit::swipe_progress(&mut paired, Some(first), 2, 0),
            None
        );

        assert_eq!(v["key"]["dropped"], serde_json::json!([2]));
        assert_eq!(v["key"]["swipes"], serde_json::json!([1, null]));
        let mut keyed = crate::visit::Swipe::default();
        let flying = issue(&mut keyed);
        let queued = issue(&mut keyed);
        keyed.note_answer(flying, "glide");
        keyed.note_answer(queued, v["key"]["replaced"].as_str().unwrap());
        keyed.retire(v["key"]["dropped"][0].as_u64().unwrap());
        assert_eq!(
            crate::visit::swipe_progress(&mut keyed, Some(queued), 3, 0),
            None
        );
        assert_eq!(
            crate::visit::swipe_progress(&mut keyed, Some(flying), 2, 0),
            Some((2, 0))
        );

        assert_eq!(v["killed"]["before"], serde_json::json!([1]));
        assert_eq!(v["killed"]["pinned"], 800);

        let swipes = v["long"]["swipes"].as_array().expect("swipes");
        assert_eq!(swipes.len(), 128);
        assert_eq!(v["long"]["answers"], "glide");
        let mut chapter = crate::visit::Swipe::default();
        for posted in swipes {
            let token = posted.as_u64().unwrap();
            let issued = issue(&mut chapter);
            assert_eq!(issued, token);
            chapter.note_answer(issued, "glide");
            let page = token as u32;
            assert_eq!(
                crate::visit::swipe_progress(&mut chapter, Some(token), page, 0),
                Some((page, 0))
            );
        }
        let late = issue(&mut chapter);
        assert!(chapter.cancel_release());
        assert_eq!(
            crate::visit::swipe_progress(&mut chapter, Some(late), 2, 50),
            None
        );
        assert_eq!(
            crate::visit::swipe_progress(&mut chapter, None, 1, 800),
            Some((1, 800))
        );
    }

    #[test]
    fn frame_floor_is_one_refresh() {
        assert!(frames_within_floor(&[8.3, 16.7, 17.0, 21.0]));
        assert!(!frames_within_floor(&[33.4]));
        assert!(!frames_within_floor(&[]));
    }

    #[test]
    fn fallbacks_stay_under_the_book() {
        let en = fallback_css("en-GB");
        assert!(en.contains("@layer spike-fallback"));
        assert!(en.contains(":where("));
        assert!(!en.contains("!important"));
        assert!(!en.contains("text-wrap"));
        assert!(en.contains("Iowan Old Style"));
        assert!(en.contains("Songti SC"));
        assert!(en.contains("22px"));
        assert!(en.contains("1.48"));
        assert!(en.contains("text-indent: 2em"));
        assert!(en.contains("-webkit-hyphenate-limit-before: 2"));
        assert!(en.contains("-webkit-hyphenate-limit-after: 3"));
        assert!(en.contains("-webkit-hyphenate-limit-lines: 2"));
        assert!(en.contains("hanging-punctuation: first allow-end last"));
        assert!(!en.contains("text-autospace"));
        let zh = fallback_css("zh-CN");
        assert!(zh.contains("20px"));
        assert!(zh.contains("1.70"));
        assert!(zh.contains("text-autospace: normal"));
        assert!(zh.contains("line-break: strict"));
        let ja = fallback_css("ja");
        assert!(ja.contains("text-autospace: normal"));
    }

    #[test]
    fn page_sheet_does_not_restyle_type() {
        let css = page_css("en");
        assert!(css.contains("overflow: hidden"));
        assert!(!css.contains("column-width: 100vw"));
        assert!(css.contains(&format!("background: {PAPER} !important")));
        let body = css
            .split("\nbody {")
            .nth(1)
            .unwrap()
            .split("\nhtml {")
            .next()
            .unwrap();
        assert!(!body.contains("background"));
        assert!(css.contains("48ch"));
        let page = css.split("#spike-footer").next().unwrap_or("");
        assert!(!page.contains("font-size"));
        assert!(!page.contains("font-family"));
        assert!(css.contains("-apple-system"));
        assert!(css.contains("tabular-nums"));
        assert!(!css.contains(".rule"));
        assert!(!css.contains("rgba(28, 25, 21, 0.28)"));
        assert!(!css.contains("text-indent"));
        assert!(!css.contains("text-align"));
        assert!(!css.contains("line-height"));
        assert!(!css.contains("hyphens"));
        let zh = page_css("zh");
        assert!(zh.contains("64ch"));
        assert!(zh.contains("11%"));
    }

    #[test]
    fn lang_is_stamped_and_the_fallback_is_first_in_the_head() {
        let html = prepare_html(
            "<html><head><style>p{font-size:19px}</style></head><body></body></html>",
            "en-GB",
        );
        assert!(html.contains("<html lang=\"en-GB\">"));
        let fallback = html.find("spike-fallback").unwrap();
        let book = html.find("font-size:19px").unwrap();
        assert!(fallback < book);
        let paper_at = html.find("spike-paper").unwrap();
        let paper_end = html[paper_at..].find("</style>").unwrap();
        let paper_rule = &html[paper_at..paper_at + paper_end];
        assert!(paper_rule.contains(PAPER));
        assert_eq!(paper_u32(), 0xffffff);
        assert_eq!(paper_rgb(), (1.0, 1.0, 1.0));
        assert!(paper_rule.contains("html, body"));
        assert!(!paper_rule.contains("!important"));
        assert!(!paper_rule.contains("div"));
        assert!(paper_at < book);
        let published = prepare_html(
            "<html><head><style>body{background:#112233}</style></head><body></body></html>",
            "en",
        );
        let default_at = published.find("id=\"spike-paper\"").unwrap();
        let book_bg = published.find("background:#112233").unwrap();
        assert!(default_at < book_bg);
        assert!(published.contains("background:#112233"));
        let replaced = prepare_html("<html lang=\"en\"><head></head></html>", "zh-Hans");
        assert!(replaced.contains("lang=\"zh-Hans\""));
        assert!(!replaced.contains("lang=\"en\""));
    }

    #[test]
    fn trim_fit_keeps_the_ratio_and_declared_size_wins() {
        let kai = trim("32kai");
        assert_eq!(trim("nope").id, "paperback");
        assert_eq!(DEFAULT_TRIM, "paperback");
        let (x, y, w, h) = fit_page(
            1100.0,
            1400.0,
            f64::from(kai.width_mm),
            f64::from(kai.height_mm),
        );
        assert!(ratio_within(w, h, 130.0, 184.0));
        assert!(x >= 0.0 && y >= 0.0);
        assert!(w <= 1100.0 + 0.01 && h <= 1400.0 + 0.01);
        let (rw, rh, source) = page_ratio(Some((1000.0, 1000.0)), trim("paperback"));
        assert_eq!(source, "declared");
        assert!(ratio_within(rw, rh, 1.0, 1.0));
        let (_, _, source) = page_ratio(None, trim("trade"));
        assert_eq!(source, "trim");
        assert_eq!(trim("trade").width_mm, 140);
    }

    #[test]
    fn justify_source_parses_and_stays_out_of_the_type_sheet() {
        let on = justify_source(true);
        let off = justify_source(false);
        assert!(on.contains("window.spikeJustify"));
        assert!(on.contains("const JUSTIFY_ON = true"));
        assert!(off.contains("const JUSTIFY_ON = false"));
        assert!(on.contains("const OWN_PUBLISHER = false"));
        assert!(!on.contains("justifyContent"));
        assert!(!on.contains("</script>"));
        assert!(!on.to_ascii_lowercase().contains("text-wrap"));
        let path = std::env::temp_dir().join("superreader-justify-check.js");
        std::fs::write(&path, &on).unwrap();
        let status = std::process::Command::new("node")
            .arg("--check")
            .arg(&path)
            .status()
            .expect("node");
        let _ = std::fs::remove_file(&path);
        assert!(status.success());
        let harvest = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../justify/harvest.test.js");
        let justify = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../justify/justify.js");
        let harvested = std::process::Command::new("node")
            .arg(&harvest)
            .arg(&justify)
            .output()
            .expect("node");
        assert!(
            harvested.status.success(),
            "{}",
            String::from_utf8_lossy(&harvested.stderr)
        );
    }

    #[test]
    fn script_is_one_slide_and_names_no_forbidden_mode() {
        let script = page_script("en-GB");
        assert!(script.contains("160"));
        assert!(script.contains("slide") || script.contains("translate3d"));
        assert!(!script.contains("__LANG__"));
        assert!(!script.contains("</script>"));
        assert!(script.contains("en-GB"));
        assert!(script.contains("\\u2060"));
        assert!(script.contains("text.charAt(i - 1) !== WJ"));
        assert!(!script.contains("!== '\\u2014'"));
        assert!(!script.contains("spike-glue"));
        assert!(script.contains("closest('#spike-footer')"));
        assert!(!script.contains("class=\"rule\""));
        assert!(!script.contains(".rule"));
        let lower = format!("{} {}", fallback_css("en"), script).to_ascii_lowercase();
        assert!(!lower.contains("night"));
        assert!(!lower.contains("theme"));
        assert!(!lower.contains("spread"));
        assert!(!lower.contains("text-wrap"));
    }

    #[test]
    fn title_fades_and_the_readme_lists_the_overrides() {
        assert_eq!(title_alpha(0), 1.0);
        assert_eq!(title_alpha(900), 0.0);
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../docs/prototypes/reading-view/README.md");
        let readme = std::fs::read_to_string(&path).unwrap_or_default();
        for (_, reason) in PAGE_OVERRIDES {
            assert!(readme.contains(reason), "missing {reason}");
        }
        assert!(readme.contains("Iowan Old Style"));
        assert!(readme.contains("Songti SC"));
        assert!(readme.contains("#ffffff"));
        assert!(!readme.contains("#f3eee4"));
        assert!(readme.contains("32kai"));
        assert!(readme.contains("paperback is the default"));
        assert!(readme.contains("130"));
        assert!(readme.contains("hanging-punctuation: first allow-end last"));
        assert!(readme.contains("text-autospace: normal"));
        assert!(readme.contains("do not set text-wrap to pretty"));
        let lower = readme.to_ascii_lowercase();
        assert!(!lower.contains("night"));
        assert!(!lower.contains("theme"));
        assert!(!lower.contains("spread"));
    }
}

const RELEASE_GLIDE_DRIVE: &str = r#"
const fs = require('fs');
const script = fs.readFileSync(process.argv[2], 'utf8');
const raf = [];
global.__now = 0;
global.requestAnimationFrame = (fn) => { raf.push(fn); return raf.length; };
global.performance = { now: () => global.__now };
global.getComputedStyle = () => ({ position: 'static', lineHeight: '32px', paddingLeft: '0px', paddingRight: '0px', fontFamily: 'serif', fontSize: '16px', fontVariant: 'normal', fontVariantNumeric: 'normal' });
function el() {
  return {
    id: '', style: { setProperty(){}, transform: 'none', cssText: '' },
    textContent: '', innerHTML: '', innerText: 'word', lang: '',
    scrollLeft: 0, scrollWidth: 1600,
    appendChild(){}, prepend(){}, remove(){},
    querySelector(){ return el(); },
    querySelectorAll(){ return []; },
    addEventListener(){},
    getContext(){ return { measureText(){ return { width: 8 }; }, fillText(){} }; },
    getBoundingClientRect(){ return { left:0, right:0, top:0, bottom:0, width:10, height:10 }; },
    closest(){ return null; },
    parentElement: null, parentNode: null, nodeType: 1, nodeValue: '',
  };
}
const body = el();
const rootEl = el();
global.document = {
  createElement: () => el(),
  documentElement: rootEl,
  head: el(),
  body,
  querySelectorAll: () => [],
  querySelector: () => null,
  getElementById: () => null,
  createTreeWalker: () => ({ nextNode(){ return null; }, currentNode: { nodeValue: '' } }),
  addEventListener(){},
  caretRangeFromPoint(){ return null; },
  createRange(){ return { setStart(){}, setEnd(){}, selectNodeContents(){}, getClientRects(){ return []; }, getBoundingClientRect(){ return { left:0, right:10, top:0, bottom:10, width:10, height:10 }; } }; },
};
global.NodeFilter = { SHOW_TEXT: 4 };
const listeners = {};
function listen(type, fn) { (listeners[type] = listeners[type] || []).push(fn); }
global.addEventListener = listen;
global.window = global;
window.innerWidth = 400;
window.innerHeight = 800;
window.webkit = { messageHandlers: { spike: { postMessage(){} } } };
function flush() {
  global.__now += 1000;
  const batch = raf.splice(0, raf.length);
  for (const fn of batch) fn(global.__now);
}
function fire(type, event) {
  for (const fn of listeners[type] || []) fn(event);
}
function boot() {
  for (const k of Object.keys(listeners)) delete listeners[k];
  raf.length = 0;
  body.scrollLeft = 0;
  rootEl.scrollLeft = 0;
  eval(script);
  flush();
}
const seen = [];
function spy() {
  const orig = window.spikeTurn;
  window.spikeTurn = function (d) { const r = orig(d); seen.push(r); return r; };
}
function take() { const out = seen.slice(); seen.length = 0; return out; }

boot(); spy();
window.spikeFollow(80);
window.spikeFollowEnd(1);
fire('keydown', { key: ' ', shiftKey: false, repeat: false, preventDefault(){} });
const space = take();
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
const arrow = take();
fire('click', { button: 0, clientX: 390, clientY: 100, preventDefault(){} });
const edge = take();
flush();
flush();
const key = { space, arrow, edge, scroll: body.scrollLeft };
fire('keydown', { key: ' ', shiftKey: true, repeat: false, preventDefault(){} });
key.after = take();

boot(); spy();
window.spikeFollow(80);
window.spikeFollowEnd(1);
window.spikeFollowDrop();
flush();
flush();
const droppedScroll = body.scrollLeft;
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
const drop = { droppedScroll, after: take(), scroll: body.scrollLeft };

boot(); spy();
window.spikeFollow(-30);
window.spikeFollowEnd(0);
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
flush();
const cancel = { during: take(), scroll: body.scrollLeft };

boot(); spy();
window.spikeFollow(80);
window.spikeFollowEnd(1);
window.spikeFollow(60);
window.spikeFollowEnd(1);
flush();
flush();
const again = { scroll: body.scrollLeft };
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
again.after = take();

boot(); spy();
window.spikeFollow(80);
window.spikeFollowEnd(1);
window.spikeFollow(20);
window.spikeFollowEnd(0);
flush();
flush();
const ignored = { scroll: body.scrollLeft };
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
ignored.after = take();

console.log(JSON.stringify({ key, drop, cancel, again, ignored }));
"#;

const REVEAL_DURING_RELEASE: &str = r#"
const fs = require('fs');
const script = fs.readFileSync(process.argv[2], 'utf8');
const raf = [];
const posts = [];
global.__now = 0;
global.requestAnimationFrame = (fn) => { raf.push(fn); return raf.length; };
global.performance = { now: () => global.__now };
global.getComputedStyle = () => ({ position: 'static', lineHeight: '32px', paddingLeft: '0px', paddingRight: '0px', fontFamily: 'serif', fontSize: '16px', fontVariant: 'normal', fontVariantNumeric: 'normal' });
function el() {
  return {
    id: '', style: { setProperty(){}, transform: 'none', cssText: '' },
    textContent: '', innerHTML: '', innerText: 'word', lang: '',
    scrollLeft: 0, scrollWidth: 1600,
    appendChild(){}, prepend(){}, remove(){},
    querySelector(){ return el(); },
    querySelectorAll(){ return []; },
    addEventListener(){},
    getContext(){ return { measureText(){ return { width: 8 }; }, fillText(){} }; },
    getBoundingClientRect(){ return { left:0, right:0, top:0, bottom:0, width:10, height:10 }; },
    closest(){ return null; },
    parentElement: null, parentNode: null, nodeType: 1, nodeValue: '',
  };
}
const body = el();
const rootEl = el();
global.document = {
  createElement: () => el(),
  documentElement: rootEl,
  head: el(),
  body,
  querySelectorAll: () => [],
  querySelector: () => null,
  getElementById: () => null,
  createTreeWalker: () => ({ nextNode(){ return null; }, currentNode: { nodeValue: '' } }),
  addEventListener(){},
  caretRangeFromPoint(){ return null; },
  createRange(){ return { setStart(){}, setEnd(){}, selectNodeContents(){}, getClientRects(){ return []; }, getBoundingClientRect(){ return { left:0, right:10, top:0, bottom:10, width:10, height:10 }; } }; },
};
global.NodeFilter = { SHOW_TEXT: 4 };
const listeners = {};
function listen(type, fn) { (listeners[type] = listeners[type] || []).push(fn); }
global.addEventListener = listen;
global.window = global;
window.innerWidth = 400;
window.innerHeight = 800;
window.webkit = { messageHandlers: { spike: { postMessage(raw){ posts.push(JSON.parse(raw)); } } } };
function flush() {
  global.__now += 1000;
  const batch = raf.splice(0, raf.length);
  for (const fn of batch) fn(global.__now);
}
function fire(type, event) {
  for (const fn of listeners[type] || []) fn(event);
}
function boot() {
  for (const k of Object.keys(listeners)) delete listeners[k];
  raf.length = 0;
  posts.length = 0;
  body.scrollLeft = 0;
  rootEl.scrollLeft = 0;
  eval(script);
  flush();
  posts.length = 0;
}
const seen = [];
function spy() {
  const orig = window.spikeTurn;
  window.spikeTurn = function (d) { const r = orig(d); seen.push(r); return r; };
}
function take() { const out = seen.slice(); seen.length = 0; return out; }

boot();
window.spikeFollow(80);
window.spikeFollowEnd(1);
window.spikeReveal(800);
flush(); flush(); flush();
const reveal = {
  scroll: body.scrollLeft,
  unpinned: posts.filter(function (p) { return p.pinned == null; }).map(function (p) { return p.page; }),
  pinned: posts.length ? posts[posts.length - 1].pinned : null
};

boot();
window.spikeFollow(80);
window.spikeFollowEnd(1);
window.spikeFollowDrop();
flush(); flush(); flush();
const drop = { scroll: body.scrollLeft, posts: posts.length };

boot(); spy();
window.spikeFollow(80);
window.spikeFollowEnd(1);
fire('keydown', { key: ' ', shiftKey: false, repeat: false, preventDefault(){} });
const during = take();
flush(); flush();
const key = { during, scroll: body.scrollLeft };
fire('keydown', { key: 'ArrowRight', repeat: false, preventDefault(){} });
key.after = take();

console.log(JSON.stringify({ reveal, drop, key }));
"#;

const COMMIT_TOKEN_DRIVE: &str = r#"
const fs = require('fs');
const script = fs.readFileSync(process.argv[2], 'utf8');
const raf = [];
const posts = [];
global.__now = 0;
global.requestAnimationFrame = (fn) => { raf.push(fn); return raf.length; };
global.performance = { now: () => global.__now };
global.getComputedStyle = () => ({ position: 'static', lineHeight: '32px', paddingLeft: '0px', paddingRight: '0px', fontFamily: 'serif', fontSize: '16px', fontVariant: 'normal', fontVariantNumeric: 'normal' });
function el() {
  return {
    id: '', style: { setProperty(){}, transform: 'none', cssText: '' },
    textContent: '', innerHTML: '', innerText: 'word', lang: '',
    scrollLeft: 0, scrollWidth: 1600,
    appendChild(){}, prepend(){}, remove(){},
    querySelector(){ return el(); },
    querySelectorAll(){ return []; },
    addEventListener(){},
    getContext(){ return { measureText(){ return { width: 8 }; }, fillText(){} }; },
    getBoundingClientRect(){ return { left:0, right:0, top:0, bottom:0, width:10, height:10 }; },
    closest(){ return null; },
    parentElement: null, parentNode: null, nodeType: 1, nodeValue: '',
  };
}
const body = el();
const rootEl = el();
global.document = {
  createElement: () => el(),
  documentElement: rootEl,
  head: el(),
  body,
  querySelectorAll: () => [],
  querySelector: () => null,
  getElementById: () => null,
  createTreeWalker: () => ({ nextNode(){ return null; }, currentNode: { nodeValue: '' } }),
  addEventListener(){},
  caretRangeFromPoint(){ return null; },
  createRange(){ return { setStart(){}, setEnd(){}, selectNodeContents(){}, getClientRects(){ return []; }, getBoundingClientRect(){ return { left:0, right:10, top:0, bottom:10, width:10, height:10 }; } }; },
};
global.NodeFilter = { SHOW_TEXT: 4 };
const listeners = {};
function listen(type, fn) { (listeners[type] = listeners[type] || []).push(fn); }
global.addEventListener = listen;
global.window = global;
window.innerWidth = 400;
window.innerHeight = 800;
window.webkit = { messageHandlers: { spike: { postMessage(raw){ posts.push(JSON.parse(raw)); } } } };
function flush() {
  global.__now += 1000;
  const batch = raf.splice(0, raf.length);
  for (const fn of batch) fn(global.__now);
}
function fire(type, event) {
  for (const fn of listeners[type] || []) fn(event);
}
function boot() {
  for (const k of Object.keys(listeners)) delete listeners[k];
  raf.length = 0;
  posts.length = 0;
  body.scrollLeft = 0;
  rootEl.scrollLeft = 0;
  eval(script);
  flush();
  posts.length = 0;
}
const seen = [];
function spy() {
  const orig = window.spikeTurn;
  window.spikeTurn = function (d) { const r = orig(d); seen.push(r); return r; };
}
function take() { const out = seen.slice(); seen.length = 0; return out; }
function swipeOf() {
  const named = posts.filter(function (p) { return p.swipe != null; });
  return named.length ? named[named.length - 1].swipe : null;
}

function keyDuringSecondFinger(key, shift) {
  boot(); spy();
  window.spikeFollow(80);
  window.spikeFollowEnd(1, 7);
  window.spikeFollow(40);
  fire('keydown', { key: key, shiftKey: !!shift, repeat: false, preventDefault(){} });
  const during = take();
  flush(); flush();
  return { during, scroll: body.scrollLeft, swipe: swipeOf() };
}

const space = keyDuringSecondFinger(' ', false);
const arrow = keyDuringSecondFinger('ArrowRight', false);
const other = keyDuringSecondFinger('a', false);

boot();
window.spikeFollow(80);
window.spikeFollowEnd(1, 7);
flush(); flush(); flush();
window.spikeFollowDrop();
const latePost = posts.filter(function (p) { return p.swipe != null; }).pop() || {};
const late = { swipe: latePost.swipe == null ? null : latePost.swipe, page: latePost.page == null ? null : latePost.page };

boot();
window.spikeFollow(80);
window.spikeReveal(800);
flush(); flush(); flush();
const reveal = {
  scroll: body.scrollLeft,
  unpinned: posts.filter(function (p) { return p.pinned == null; }).map(function (p) { return p.page; }),
  pinned: posts.length ? posts[posts.length - 1].pinned : null
};

console.log(JSON.stringify({ space, arrow, other, late, reveal }));
"#;

const COMMIT_CHAIN_DRIVE: &str = r#"
const fs = require('fs');
const script = fs.readFileSync(process.argv[2], 'utf8');
const raf = [];
const posts = [];
global.__now = 0;
global.requestAnimationFrame = (fn) => { raf.push(fn); return raf.length; };
global.performance = { now: () => global.__now };
global.getComputedStyle = () => ({ position: 'static', lineHeight: '32px', paddingLeft: '0px', paddingRight: '0px', fontFamily: 'serif', fontSize: '16px', fontVariant: 'normal', fontVariantNumeric: 'normal' });
function el() {
  return {
    id: '', style: { setProperty(){}, transform: 'none', cssText: '' },
    textContent: 'word', innerHTML: '', innerText: 'word', lang: '',
    scrollLeft: 0, scrollWidth: 80000,
    appendChild(child){ if (child) child.parentNode = this; }, prepend(){}, remove(){},
    querySelector(){ return null; }, querySelectorAll(){ return []; },
    addEventListener(){},
    getContext(){ return { measureText(){ return { width: 8 }; }, fillText(){} }; },
    getBoundingClientRect(){ return { left:0, right:0, top:0, bottom:0, width:10, height:10 }; },
    closest(){ return null; }, parentElement: null, parentNode: null, nodeType: 1, nodeValue: '',
  };
}
const body = el();
const rootEl = el();
rootEl.scrollWidth = 80000;
body.scrollWidth = 80000;
global.document = {
  createElement: () => el(), documentElement: rootEl, head: el(), body,
  querySelectorAll: () => [], querySelector: () => null, getElementById: () => null,
  createTreeWalker: () => ({ nextNode(){ return null; }, currentNode: { nodeValue: '' } }),
  addEventListener(){}, caretRangeFromPoint(){ return null; },
  createRange(){ return { setStart(){}, setEnd(){}, selectNodeContents(){}, getClientRects(){ return []; }, getBoundingClientRect(){ return { left:0, right:10, top:0, bottom:10, width:10, height:10 }; } }; },
};
global.NodeFilter = { SHOW_TEXT: 4 };
const listeners = {};
function listen(type, fn) { (listeners[type] = listeners[type] || []).push(fn); }
global.addEventListener = listen;
global.window = global;
window.innerWidth = 400;
window.innerHeight = 800;
window.webkit = { messageHandlers: { spike: { postMessage(raw){ posts.push(JSON.parse(raw)); } } } };
function flush() {
  global.__now += 1000;
  const batch = raf.splice(0, raf.length);
  for (const fn of batch) fn(global.__now);
}
function fire(type, event) {
  for (const fn of listeners[type] || []) fn(event);
}
function boot() {
  for (const k of Object.keys(listeners)) delete listeners[k];
  raf.length = 0;
  posts.length = 0;
  body.scrollLeft = 0;
  rootEl.scrollLeft = 0;
  eval(script);
  flush();
  posts.length = 0;
}
function swipeOf(list) {
  return list.map(function (p) { return p.swipe == null ? null : p.swipe; });
}

boot();
const firstAnswer = window.spikeFollowEnd(1, 1);
const secondAnswer = window.spikeFollowEnd(1, 2);
flush(); flush(); flush();
const second = { first: firstAnswer, second: secondAnswer, swipes: swipeOf(posts), pages: posts.map(function (p) { return p.page; }) };

boot();
const queuedAnswer = window.spikeFollowEnd(1, 1);
const replaced = window.spikeFollowEnd(1, 2);
fire('keydown', { key: ' ', shiftKey: false, repeat: false, preventDefault(){} });
const dropped = posts.filter(function (p) { return p.dropped; }).map(function (p) { return p.swipe; });
flush(); flush(); flush();
const kept = posts.filter(function (p) { return !p.dropped; });
const key = { queuedAnswer, replaced, dropped, swipes: swipeOf(kept), pages: kept.map(function (p) { return p.page; }) };

boot();
window.spikeFollowEnd(1, 1);
window.spikeFollowEnd(1, 2);
flush();
window.spikeFollowDrop();
flush(); flush();
const beforeReveal = posts.slice();
window.spikeReveal(800);
flush(); flush(); flush();
const killed = {
  before: swipeOf(beforeReveal),
  pages: beforeReveal.map(function (p) { return p.page; }),
  pinned: posts.length ? posts[posts.length - 1].pinned : null
};

boot();
const answers = [];
for (let i = 1; i <= 128; i++) {
  answers.push(window.spikeFollowEnd(1, i));
  flush();
  flush();
}
const swipes = posts.map(function (p) { return p.swipe; }).filter(function (s) { return s != null; });
const long = { answers: answers[0], count: swipes.length, first: swipes[0], last: swipes[swipes.length - 1], swipes: swipes };

console.log(JSON.stringify({ second, key, killed, long }));
"#;
