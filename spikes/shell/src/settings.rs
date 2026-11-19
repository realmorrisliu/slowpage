use std::path::{Path, PathBuf};

use crate::reading;

pub fn path() -> PathBuf {
    support_dir().join("settings.json")
}

/// Verify writes this when a launch cannot see `SUPERREADER_SUPPORT_DIR`.
const SUPPORT_FILE: &str = "/tmp/superreader-spike/support-dir";

pub fn support_dir() -> PathBuf {
    if let Some(dir) = configured_support_dir() {
        return dir;
    }
    default_support_dir()
}

/// Settings and history. A set env wins, and a blank env means
/// the real directory. The file is the same override for a process that was
/// not given the env, such as `open -a`.
pub fn configured_support_dir() -> Option<PathBuf> {
    match std::env::var("SUPERREADER_SUPPORT_DIR") {
        Ok(value) => return trimmed_dir(value.trim()),
        Err(_) => {}
    }
    let text = std::fs::read_to_string(SUPPORT_FILE).unwrap_or_default();
    trimmed_dir(text.trim())
}

pub fn glyph_cache_dir() -> PathBuf {
    support_dir().join("shell-glyphs")
}

fn trimmed_dir(value: &str) -> Option<PathBuf> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(PathBuf::from(value))
    }
}

pub(crate) fn default_support_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".into());
    PathBuf::from(home).join("Library/Application Support/app.slowpage.reader")
}

fn read_value(path: &Path) -> serde_json::Value {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .filter(|value: &serde_json::Value| value.is_object())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn write_value(path: &Path, value: &serde_json::Value) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    }
    let body = serde_json::to_string(value).map_err(|err| err.to_string())?;
    std::fs::write(path, format!("{body}\n")).map_err(|err| err.to_string())
}

pub fn load(path: &Path) -> String {
    let value = read_value(path);
    let id = value
        .get("trim")
        .and_then(|item| item.as_str())
        .unwrap_or("");
    reading::trim(id).id.to_string()
}

pub fn bold_word_starts(path: &Path) -> bool {
    read_value(path)
        .get("boldWordStarts")
        .and_then(|item| item.as_bool())
        .unwrap_or(false)
}

pub fn store(path: &Path, id: &str) -> Result<(), String> {
    let id = reading::trim(id).id;
    let mut value = read_value(path);
    value["trim"] = serde_json::json!(id);
    write_value(path, &value)
}

pub struct Place {
    pub path: String,
    pub page: u32,
}

pub fn load_place(path: &Path) -> Option<Place> {
    let value = read_value(path);
    let place = value.get("place")?;
    let book = place.get("path").and_then(|item| item.as_str())?;
    let page = place.get("page").and_then(|item| item.as_u64())?;
    if book.is_empty() || page == 0 || !Path::new(book).exists() {
        return None;
    }
    if crate::books::parse_book(book).is_none() {
        return None;
    }
    Some(Place {
        path: book.to_string(),
        page: page as u32,
    })
}

pub fn store_place(path: &Path, book: &str, page: u32) -> Result<(), String> {
    let mut value = read_value(path);
    value["place"] = serde_json::json!({ "path": book, "page": page });
    write_value(path, &value)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ReadingRecord {
    pub path: String,
    pub page: u32,
    pub total: Option<u32>,
    pub opened_at: i64,
    pub offset: u32,
    pub href: String,
    pub mark: Option<String>,
    pub opened_label: Option<String>,
    /// A caret hit, including character 0. Absent in the file means a page-only row.
    pub caret: bool,
}

pub struct StoredList {
    pub label: String,
    pub count: u32,
    pub color: u32,
}

pub struct StoredChat {
    pub kind: String,
    pub text: String,
}

/// The page already stored for this book. A book with no row opens at 1.
pub fn resume_page(path: &Path, book: &str) -> u32 {
    if let Some(row) = load_history(path).into_iter().find(|row| row.path == book) {
        return row.page;
    }
    load_place(path)
        .filter(|place| place.path == book)
        .map(|place| place.page)
        .unwrap_or(1)
}

/// The character offset stored with the page. A missing offset opens at the page start.
pub fn resume_offset(path: &Path, book: &str) -> u32 {
    load_history(path)
        .into_iter()
        .find(|row| row.path == book)
        .map(|row| row.offset)
        .unwrap_or(0)
}

pub fn resume_caret(path: &Path, book: &str) -> bool {
    load_history(path)
        .into_iter()
        .find(|row| row.path == book)
        .is_some_and(|row| row.caret)
}

pub fn resume_href(path: &Path, book: &str) -> String {
    load_history(path)
        .into_iter()
        .find(|row| row.path == book)
        .map(|row| row.href)
        .unwrap_or_default()
}

pub fn load_history(path: &Path) -> Vec<ReadingRecord> {
    let value = read_value(path);
    let Some(rows) = value.get("history").and_then(|item| item.as_array()) else {
        return Vec::new();
    };
    rows.iter().filter_map(record_from_value).collect()
}

fn record_from_value(value: &serde_json::Value) -> Option<ReadingRecord> {
    let path = value.get("path").and_then(|item| item.as_str())?;
    let page = value.get("page").and_then(|item| item.as_u64())?;
    if path.is_empty() || page == 0 {
        return None;
    }
    let total = value
        .get("total")
        .and_then(|item| item.as_u64())
        .map(|total| total as u32)
        .filter(|total| *total > 0);
    let opened_at = value
        .get("openedAt")
        .and_then(|item| item.as_i64())
        .unwrap_or(0);
    let mark = value
        .get("mark")
        .and_then(|item| item.as_str())
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    let opened_label = value
        .get("openedLabel")
        .and_then(|item| item.as_str())
        .filter(|text| !text.is_empty())
        .map(str::to_string);
    Some(ReadingRecord {
        path: path.to_string(),
        page: page as u32,
        total,
        opened_at,
        offset: value
            .get("offset")
            .and_then(|item| item.as_u64())
            .map(|offset| offset as u32)
            .unwrap_or(0),
        href: value
            .get("href")
            .and_then(|item| item.as_str())
            .unwrap_or("")
            .to_string(),
        mark,
        opened_label,
        caret: value.get("caret").and_then(|item| item.as_bool()) == Some(true),
    })
}

/// Inserts or replaces the book's row and stamps `opened_at`. Also writes `place`.
pub fn record_open(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    opened_at: i64,
) -> Result<(), String> {
    write_history(path, book, page, total, Some(opened_at), None, None, None)
}

/// Updates the stored page. Keeps the previous open time and a known total.
/// The book row `handle_page` writes. An empty book or page 0 is a failed
/// write: `record_progress` would return `Ok` and leave the file alone.
pub struct SettingsBook<'a> {
    pub file: &'a std::path::Path,
    pub book: &'a str,
}

impl crate::visit::PlaceFile for SettingsBook<'_> {
    fn write_place(&mut self, place: &crate::visit::ReadingPlace) -> bool {
        if self.book.is_empty() || place.page == 0 {
            return false;
        }
        record_progress(
            self.file,
            self.book,
            place.page,
            None,
            place.offset,
            &place.href,
        )
        .is_ok()
    }

    fn write_anchor(&mut self, place: &crate::visit::ReadingPlace) -> bool {
        if self.book.is_empty() || place.page == 0 {
            return false;
        }
        record_anchor(
            self.file,
            self.book,
            place.page,
            None,
            place.offset,
            &place.href,
        )
        .is_ok()
    }

    fn resume_caret(&self) -> bool {
        resume_caret(self.file, self.book)
    }

    fn resume(&self) -> crate::visit::ReadingPlace {
        crate::visit::ReadingPlace {
            href: resume_href(self.file, self.book),
            page: resume_page(self.file, self.book),
            offset: resume_offset(self.file, self.book),
        }
    }
}

pub fn record_progress(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    offset: u32,
    href: &str,
) -> Result<(), String> {
    write_history(
        path,
        book,
        page,
        total,
        None,
        Some(offset),
        Some(href),
        Some(false),
    )
}

pub fn record_anchor(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    offset: u32,
    href: &str,
) -> Result<(), String> {
    write_history(
        path,
        book,
        page,
        total,
        None,
        Some(offset),
        Some(href),
        Some(true),
    )
}

fn write_history(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    opened_at: Option<i64>,
    offset: Option<u32>,
    href: Option<&str>,
    caret: Option<bool>,
) -> Result<(), String> {
    if book.is_empty() || page == 0 {
        return Ok(());
    }
    let mut value = read_value(path);
    let mut rows = load_history(path);
    if let Some(existing) = rows.iter_mut().find(|row| row.path == book) {
        existing.page = page;
        if total.is_some() {
            existing.total = total;
        }
        if let Some(stamp) = opened_at {
            existing.opened_at = stamp;
        }
        if let Some(offset) = offset {
            existing.offset = offset;
        }
        if let Some(href) = href {
            existing.href = href.to_string();
        }
        if let Some(caret) = caret {
            existing.caret = caret;
        }
    } else {
        rows.push(ReadingRecord {
            path: book.to_string(),
            page,
            total,
            opened_at: opened_at.unwrap_or(0),
            offset: offset.unwrap_or(0),
            href: href.unwrap_or("").to_string(),
            mark: None,
            opened_label: None,
            caret: caret.unwrap_or(false),
        });
    }
    value["history"] = serde_json::json!(rows.iter().map(history_value).collect::<Vec<_>>());
    value["place"] = serde_json::json!({ "path": book, "page": page });
    write_value(path, &value)
}

fn history_value(row: &ReadingRecord) -> serde_json::Value {
    let mut value = serde_json::json!({
        "path": row.path,
        "page": row.page,
        "total": row.total,
        "openedAt": row.opened_at,
        "offset": row.offset,
    });
    if !row.href.is_empty() {
        value["href"] = serde_json::json!(row.href);
    }
    if let Some(mark) = &row.mark {
        value["mark"] = serde_json::json!(mark);
    }
    if let Some(label) = &row.opened_label {
        value["openedLabel"] = serde_json::json!(label);
    }
    if row.caret {
        value["caret"] = serde_json::json!(true);
    }
    value
}

pub fn record_marked(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    opened_at: i64,
    mark: &str,
    opened_label: &str,
) -> Result<(), String> {
    if book.is_empty() || page == 0 {
        return Ok(());
    }
    let mut value = read_value(path);
    let mut rows = load_history(path);
    if let Some(existing) = rows.iter_mut().find(|row| row.path == book) {
        existing.page = page;
        existing.total = total;
        existing.opened_at = opened_at;
        existing.mark = Some(mark.to_string());
        existing.opened_label = Some(opened_label.to_string());
    } else {
        rows.push(ReadingRecord {
            path: book.to_string(),
            page,
            total,
            opened_at,
            offset: 0,
            href: String::new(),
            mark: Some(mark.to_string()),
            opened_label: Some(opened_label.to_string()),
            caret: false,
        });
    }
    value["history"] = serde_json::json!(rows.iter().map(history_value).collect::<Vec<_>>());
    write_value(path, &value)
}

pub fn load_lists(path: &Path) -> Vec<StoredList> {
    let value = read_value(path);
    let Some(rows) = value.get("lists").and_then(|item| item.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let label = row.get("label")?.as_str()?.to_string();
            if label.is_empty() {
                return None;
            }
            let count = row.get("count").and_then(|item| item.as_u64()).unwrap_or(0) as u32;
            let color = row
                .get("color")
                .and_then(|item| item.as_str())
                .and_then(parse_hex)
                .unwrap_or(0x008E_8E93);
            Some(StoredList {
                label,
                count,
                color,
            })
        })
        .collect()
}

pub fn load_chat(path: &Path) -> Vec<StoredChat> {
    let value = read_value(path);
    let Some(rows) = value.get("chat").and_then(|item| item.as_array()) else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|row| {
            let kind = row.get("kind")?.as_str()?.to_string();
            let text = row.get("text")?.as_str()?.to_string();
            if text.is_empty() {
                return None;
            }
            Some(StoredChat { kind, text })
        })
        .collect()
}

pub fn store_form_chrome(
    path: &Path,
    lists: &[StoredList],
    chat: &[StoredChat],
) -> Result<(), String> {
    let mut value = read_value(path);
    if value.get("lists").is_none() {
        value["lists"] = serde_json::json!(lists
            .iter()
            .map(|row| serde_json::json!({
                "label": row.label,
                "count": row.count,
                "color": format!("#{:06X}", row.color & 0x00FF_FFFF),
            }))
            .collect::<Vec<_>>());
    }
    if value.get("chat").is_none() {
        value["chat"] = serde_json::json!(chat
            .iter()
            .map(|row| serde_json::json!({ "kind": row.kind, "text": row.text }))
            .collect::<Vec<_>>());
    }
    write_value(path, &value)
}

fn parse_hex(text: &str) -> Option<u32> {
    let hex = text.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    u32::from_str_radix(hex, 16).ok()
}

pub fn library_folder(path: &Path) -> Option<PathBuf> {
    let value = read_value(path);
    let folder = value.get("libraryFolder").and_then(|item| item.as_str())?;
    let folder = PathBuf::from(folder);
    if folder.is_dir() {
        Some(folder)
    } else {
        None
    }
}

pub fn store_library_folder(path: &Path, folder: &Path) -> Result<(), String> {
    let mut value = read_value(path);
    value["libraryFolder"] = serde_json::json!(folder.display().to_string());
    write_value(path, &value)
}

pub fn store_bold(path: &Path, on: bool) -> Result<(), String> {
    let mut value = read_value(path);
    if value.get("trim").and_then(|item| item.as_str()).is_none() {
        value["trim"] = serde_json::json!(reading::DEFAULT_TRIM);
    }
    value["boldWordStarts"] = serde_json::json!(on);
    write_value(path, &value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visit::PlaceFile;

    #[test]
    fn round_trip_keeps_a_known_trim_and_rejects_unknown() {
        let dir = std::env::temp_dir().join(format!("superreader-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        assert_eq!(load(&file), "paperback");
        store(&file, "32kai").unwrap();
        assert_eq!(load(&file), "32kai");
        store(&file, "nope").unwrap();
        assert_eq!(load(&file), "paperback");
        assert!(!bold_word_starts(&file));
        store_bold(&file, true).unwrap();
        store(&file, "trade").unwrap();
        assert!(bold_word_starts(&file));
        assert_eq!(load(&file), "trade");
        store_bold(&file, false).unwrap();
        assert!(!bold_word_starts(&file));
        assert_eq!(load(&file), "trade");
        let book = dir.join("sample.epub");
        std::fs::write(&book, "epub").unwrap();
        store_place(&file, book.to_str().unwrap(), 4).unwrap();
        let place = load_place(&file).unwrap();
        assert_eq!(place.path, book.to_str().unwrap());
        assert_eq!(place.page, 4);
        assert_eq!(load(&file), "trade");
        assert!(!bold_word_starts(&file));
        store_place(&file, book.to_str().unwrap(), 0).unwrap();
        assert!(load_place(&file).is_none());
        record_open(&file, book.to_str().unwrap(), 2, None, 10).unwrap();
        record_open(&file, book.to_str().unwrap(), 4, Some(20), 30).unwrap();
        let history = load_history(&file);
        assert_eq!(history.len(), 1);
        assert_eq!(history[0].page, 4);
        assert_eq!(history[0].total, Some(20));
        assert_eq!(history[0].opened_at, 30);
        record_progress(&file, book.to_str().unwrap(), 5, None, 0, "").unwrap();
        let history = load_history(&file);
        assert_eq!(history[0].page, 5);
        assert_eq!(history[0].total, Some(20));
        assert_eq!(history[0].opened_at, 30);
        assert_eq!(load_place(&file).unwrap().page, 5);
        let later = dir.join("later.epub");
        std::fs::write(&later, "epub").unwrap();
        record_open(&file, later.to_str().unwrap(), 2, Some(8), 20).unwrap();
        assert!(load_place(&file).unwrap().path.ends_with("later.epub"));
        let again = resume_page(&file, book.to_str().unwrap());
        assert_eq!(again, 5);
        record_open(&file, book.to_str().unwrap(), again, None, 40).unwrap();
        let history = load_history(&file);
        let earlier = history
            .iter()
            .find(|row| row.path == book.to_str().unwrap())
            .unwrap();
        assert_eq!(earlier.page, 5);
        assert_eq!(earlier.total, Some(20));
        assert_eq!(earlier.opened_at, 40);
        let other = history
            .iter()
            .find(|row| row.path.ends_with("later.epub"))
            .unwrap();
        assert_eq!(other.page, 2);
        assert_eq!(other.opened_at, 20);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn support_dir_follows_a_verify_override_and_ignores_a_blank_one() {
        let dir = std::env::temp_dir().join(format!("superreader-support-{}", std::process::id()));
        let previous = std::env::var("SUPERREADER_SUPPORT_DIR").ok();
        std::env::set_var("SUPERREADER_SUPPORT_DIR", &dir);
        assert_eq!(support_dir(), dir);
        std::env::set_var("SUPERREADER_SUPPORT_DIR", " ");
        assert!(support_dir().ends_with("app.slowpage.reader"));
        match previous {
            Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
            None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
        }
    }

    #[test]
    fn glyph_cache_stays_inside_the_support_override() {
        let dir = std::env::temp_dir().join(format!("superreader-glyphs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let previous = std::env::var("SUPERREADER_SUPPORT_DIR").ok();
        let previous_file = std::fs::read_to_string(SUPPORT_FILE).ok();
        std::env::remove_var("SUPERREADER_SUPPORT_DIR");
        std::fs::create_dir_all("/tmp/superreader-spike").unwrap();
        std::fs::write(SUPPORT_FILE, dir.to_string_lossy().as_bytes()).unwrap();
        let cache = glyph_cache_dir();
        assert!(cache.starts_with(&dir), "{cache:?}");
        let real = default_support_dir().join("shell-glyphs");
        assert_ne!(cache, real);
        std::fs::create_dir_all(&cache).unwrap();
        let name = format!("probe-{}.png", std::process::id());
        std::fs::write(cache.join(&name), b"png").unwrap();
        assert!(!real.join(&name).exists());
        let _ = std::fs::remove_dir_all(&dir);
        match previous_file {
            Some(value) => {
                let _ = std::fs::write(SUPPORT_FILE, value);
            }
            None => {
                let _ = std::fs::remove_file(SUPPORT_FILE);
            }
        }
        match previous {
            Some(value) => std::env::set_var("SUPERREADER_SUPPORT_DIR", value),
            None => std::env::remove_var("SUPERREADER_SUPPORT_DIR"),
        }
    }

    #[test]
    fn a_stored_sentence_round_trips() {
        let dir = std::env::temp_dir().join(format!("superreader-sentence-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = dir.join("sample.epub");
        std::fs::write(&book, "epub").unwrap();
        let path = book.to_str().unwrap();
        record_open(&file, path, 4, Some(10), 1).unwrap();
        assert_eq!(resume_offset(&file, path), 0);
        record_progress(&file, path, 4, Some(10), 800, "/OEBPS/next.xhtml").unwrap();
        assert_eq!(resume_page(&file, path), 4);
        assert_eq!(resume_offset(&file, path), 800);
        assert_eq!(resume_href(&file, path), "/OEBPS/next.xhtml");
        record_open(&file, path, 4, Some(10), 2).unwrap();
        assert_eq!(resume_offset(&file, path), 800);
        assert_eq!(resume_href(&file, path), "/OEBPS/next.xhtml");
        record_progress(&file, path, 5, Some(10), 20, "/OEBPS/chapter.xhtml").unwrap();
        assert_eq!(resume_page(&file, path), 5);
        assert_eq!(resume_offset(&file, path), 20);
        assert_eq!(resume_href(&file, path), "/OEBPS/chapter.xhtml");

        let legacy = serde_json::json!({
            "history": [{
                "path": path,
                "page": 4,
                "offset": 800,
                "openedAt": 3
            }]
        });
        std::fs::write(&file, legacy.to_string()).unwrap();
        assert_eq!(resume_href(&file, path), "");
        assert_eq!(resume_page(&file, path), 4);
        assert_eq!(resume_offset(&file, path), 800);
        let spine = [
            "/OEBPS/chapter.xhtml".to_string(),
            "/OEBPS/empty.xhtml".to_string(),
            "/OEBPS/next.xhtml".to_string(),
        ];
        let kept = crate::visit::opened_place(&spine, &resume_href(&file, path), 4, 800);
        assert_eq!(kept.page, 4);
        assert_eq!(kept.offset, 800);
        assert_eq!(kept.index, 0);
        assert_eq!(
            crate::visit::chapter_target(
                &resume_href(&file, path),
                &spine[0],
                4,
                800,
                &spine[0],
                Some(0)
            ),
            (4, 800)
        );

        record_progress(&file, path, 4, Some(10), 800, "/gone.xhtml").unwrap();
        let dropped = crate::visit::opened_place(&spine, &resume_href(&file, path), 4, 800);
        assert_eq!(dropped.index, 0);
        assert_eq!(dropped.page, 1);
        assert_eq!(dropped.offset, 0);
        record_progress(
            &file,
            path,
            dropped.page,
            Some(10),
            dropped.offset,
            &spine[dropped.index],
        )
        .unwrap();
        assert_eq!(resume_page(&file, path), 1);
        assert_eq!(resume_offset(&file, path), 0);
        assert_eq!(resume_href(&file, path), "/OEBPS/chapter.xhtml");

        let valid = crate::visit::opened_place(&spine, "/OEBPS/empty.xhtml", 2, 20);
        assert_eq!(valid.index, 1);
        assert_eq!(valid.page, 2);
        assert_eq!(valid.offset, 20);

        record_progress(&file, path, 4, Some(10), 800, "/OEBPS/chapter.xhtml").unwrap();
        let chapter = "/OEBPS/chapter.xhtml";
        let next = "/OEBPS/next.xhtml";
        let empty = "/OEBPS/empty.xhtml";
        let waiting = crate::visit::step_restore(true, None, None, 4, None, false);
        assert!(!waiting.write);
        let origin = crate::visit::ReadingPlace {
            href: chapter.to_string(),
            page: 4,
            offset: 800,
        };
        let marks = crate::visit::contents_marks("sample", &spine);
        let selected = crate::visit::chapter_jump(&origin, &marks, 2, &spine, None);
        let crate::visit::ChapterMove::Chapter { index, offset } = selected.go else {
            panic!("selecting another chapter moves");
        };
        assert_eq!(offset, 0);
        let chosen = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 800,
            jump_href: &spine[index],
            jump_offset: Some(offset),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: &spine[index],
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!((chosen.saved_page, chosen.saved_offset), (1, 0));
        assert!(chosen.record);
        record_progress(
            &file,
            path,
            chosen.saved_page,
            Some(10),
            chosen.saved_offset,
            &chosen.saved_href,
        )
        .unwrap();
        assert_eq!(resume_href(&file, path), next);
        assert_eq!(resume_page(&file, path), 1);
        assert_eq!(resume_offset(&file, path), 0);

        let returned = crate::visit::chapter_return(
            &crate::visit::ReadingPlace {
                href: next.to_string(),
                page: 1,
                offset: 0,
            },
            Some(origin),
            &spine,
        );
        let crate::visit::ChapterMove::Chapter {
            index: back_index,
            offset: back_offset,
        } = returned.go
        else {
            panic!("returning across chapters moves");
        };
        assert_eq!(back_offset, 800);
        let ready_first = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: next,
            saved_page: 1,
            saved_offset: 0,
            jump_href: &spine[back_index],
            jump_offset: Some(back_offset),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: &spine[back_index],
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!(ready_first.reveal_jump, Some(800));
        assert_eq!(ready_first.jump_offset, None);
        assert!(!ready_first.pin_write);
        assert_eq!(ready_first.saved_offset, 800);
        assert!(ready_first.record);
        record_progress(
            &file,
            path,
            ready_first.saved_page,
            Some(10),
            ready_first.saved_offset,
            &ready_first.saved_href,
        )
        .unwrap();
        assert_eq!(resume_href(&file, path), chapter);
        assert_eq!(resume_page(&file, path), 1);
        assert_eq!(resume_offset(&file, path), 800);
        let early = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: next,
            saved_page: 1,
            saved_offset: 0,
            jump_href: &spine[back_index],
            jump_offset: Some(back_offset),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: &spine[back_index],
            ready: false,
            reported: false,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!(early.reveal_jump, None);
        assert_eq!(early.jump_offset, Some(800));
        assert_eq!(early.saved_offset, 800);
        let before_pin = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &ready_first.saved_href,
            saved_page: ready_first.saved_page,
            saved_offset: ready_first.saved_offset,
            jump_href: chapter,
            jump_offset: ready_first.jump_offset,
            awaiting: ready_first.awaiting,
            pending: ready_first.pending,
            hold: ready_first.hold,
            shown: 1,
            href: chapter,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert!(!before_pin.record);
        assert_eq!(before_pin.saved_offset, 800);
        let stale = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 800,
            jump_href: chapter,
            jump_offset: Some(800),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: next,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!(stale.reveal_jump, None);
        assert_eq!(stale.saved_offset, 0);
        let here = crate::visit::ReadingPlace {
            href: ready_first.saved_href.clone(),
            page: ready_first.saved_page,
            offset: ready_first.saved_offset,
        };
        let again = crate::visit::chapter_jump(&here, &marks, 1, &spine, None);
        assert_eq!(again.back.as_ref().map(|place| place.offset), Some(800));
        let crate::visit::ChapterMove::Chapter {
            index: next_index,
            offset: next_offset,
        } = again.go
        else {
            panic!("leaving before the pin moves");
        };
        assert_eq!(next_offset, 0);
        let left = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &here.href,
            saved_page: here.page,
            saved_offset: here.offset,
            jump_href: &spine[next_index],
            jump_offset: Some(next_offset),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 1,
            href: &spine[next_index],
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!((left.saved_page, left.saved_offset), (1, 0));
        record_progress(
            &file,
            path,
            left.saved_page,
            Some(10),
            left.saved_offset,
            &left.saved_href,
        )
        .unwrap();
        assert_eq!(resume_href(&file, path), empty);
        assert_eq!(resume_page(&file, path), 1);
        assert_eq!(resume_offset(&file, path), 0);

        record_progress(&file, path, 4, Some(10), 800, chapter).unwrap();
        let missed = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 800,
            jump_href: empty,
            jump_offset: Some(0),
            awaiting: None,
            pending: 400,
            hold: None,
            shown: 1,
            href: empty,
            ready: true,
            reported: true,
            pinned: Some(0),
            missed: true,
            sentence: 0,
            still_turning: false,
        });
        assert!(!missed.pin_write);
        assert_eq!(missed.reveal_pending, None);
        assert_eq!(missed.awaiting, None);
        assert_eq!((missed.saved_page, missed.saved_offset), (1, 0));
        record_progress(
            &file,
            path,
            missed.saved_page,
            Some(10),
            missed.saved_offset,
            &missed.saved_href,
        )
        .unwrap();
        assert_eq!(resume_href(&file, path), empty);
        assert_eq!(resume_page(&file, path), 1);
        assert_eq!(resume_offset(&file, path), 0);
        let same = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 800,
            jump_href: "",
            jump_offset: None,
            awaiting: Some(800),
            pending: 0,
            hold: None,
            shown: 4,
            href: chapter,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 800,
            still_turning: false,
        });
        assert!(!same.pin_write);
        assert!(!same.record);
        assert_eq!((same.saved_page, same.saved_offset), (4, 800));
        let legacy = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: "",
            saved_page: 4,
            saved_offset: 800,
            jump_href: chapter,
            jump_offset: Some(0),
            awaiting: None,
            pending: 0,
            hold: None,
            shown: 4,
            href: chapter,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!((legacy.saved_page, legacy.saved_offset), (4, 800));
        let resume = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 400,
            jump_href: next,
            jump_offset: Some(0),
            awaiting: None,
            pending: 400,
            hold: None,
            shown: 1,
            href: next,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let resume_pin = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &resume.saved_href,
            saved_page: resume.saved_page,
            saved_offset: resume.saved_offset,
            jump_href: next,
            jump_offset: resume.jump_offset,
            awaiting: resume.awaiting,
            pending: resume.pending,
            hold: resume.hold,
            shown: 1,
            href: next,
            ready: true,
            reported: true,
            pinned: Some(400),
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let back_resume = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: next,
            saved_page: 1,
            saved_offset: 0,
            jump_href: chapter,
            jump_offset: Some(800),
            awaiting: None,
            pending: 400,
            hold: None,
            shown: 1,
            href: chapter,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let back_pin = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &back_resume.saved_href,
            saved_page: back_resume.saved_page,
            saved_offset: back_resume.saved_offset,
            jump_href: chapter,
            jump_offset: back_resume.jump_offset,
            awaiting: back_resume.awaiting,
            pending: back_resume.pending,
            hold: back_resume.hold,
            shown: 1,
            href: chapter,
            ready: true,
            reported: true,
            pinned: Some(400),
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let marks_now = crate::visit::contents_marks("sample", &spine);
        let to_next = crate::visit::navigate(crate::visit::NavIn {
            shown_href: chapter,
            shown_page: 4,
            saved_href: chapter,
            saved_page: 4,
            saved_offset: 800,
            jump_href: "",
            jump_offset: None,
            awaiting: None,
            pending: 400,
            back: None,
            marks: &marks_now,
            spine: &spine,
            pick: 2,
        });
        assert_eq!(to_next.pending, 0);
        assert_eq!(to_next.saved_offset, 0);
        assert_eq!(to_next.back.as_ref().map(|place| place.offset), Some(800));
        record_progress(
            &file,
            path,
            to_next.saved_page,
            Some(10),
            to_next.saved_offset,
            &to_next.saved_href,
        )
        .unwrap();
        assert_eq!(resume_offset(&file, path), 0);
        let to_empty = crate::visit::navigate(crate::visit::NavIn {
            shown_href: &to_next.jump_href,
            shown_page: 1,
            saved_href: &to_next.saved_href,
            saved_page: to_next.saved_page,
            saved_offset: to_next.saved_offset,
            jump_href: &to_next.jump_href,
            jump_offset: to_next.jump_offset,
            awaiting: to_next.awaiting,
            pending: to_next.pending,
            back: to_next.back.clone(),
            marks: &marks_now,
            spine: &spine,
            pick: 1,
        });
        assert_eq!(
            to_empty.back.as_ref().map(|place| place.href.as_str()),
            Some(next)
        );
        assert_eq!(to_empty.back.as_ref().map(|place| place.offset), Some(0));
        assert_eq!(to_empty.saved_offset, 0);
        record_progress(
            &file,
            path,
            to_empty.saved_page,
            Some(10),
            to_empty.saved_offset,
            &to_empty.saved_href,
        )
        .unwrap();
        let back_to_next = crate::visit::navigate(crate::visit::NavIn {
            shown_href: &to_empty.jump_href,
            shown_page: 1,
            saved_href: &to_empty.saved_href,
            saved_page: to_empty.saved_page,
            saved_offset: to_empty.saved_offset,
            jump_href: &to_empty.jump_href,
            jump_offset: to_empty.jump_offset,
            awaiting: to_empty.awaiting,
            pending: 400,
            back: to_empty.back.clone(),
            marks: &marks_now,
            spine: &spine,
            pick: -1,
        });
        assert_eq!(back_to_next.pending, 0);
        assert_eq!(back_to_next.jump_offset, Some(0));
        assert_eq!(back_to_next.saved_href, next);
        assert_eq!(back_to_next.saved_offset, 0);
        let returned = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &back_to_next.saved_href,
            saved_page: back_to_next.saved_page,
            saved_offset: back_to_next.saved_offset,
            jump_href: &back_to_next.jump_href,
            jump_offset: back_to_next.jump_offset,
            awaiting: back_to_next.awaiting,
            pending: back_to_next.pending,
            hold: None,
            shown: 1,
            href: &back_to_next.jump_href,
            ready: false,
            reported: false,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let returned_ready = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &returned.saved_href,
            saved_page: returned.saved_page,
            saved_offset: returned.saved_offset,
            jump_href: &back_to_next.jump_href,
            jump_offset: returned.jump_offset,
            awaiting: returned.awaiting,
            pending: returned.pending,
            hold: returned.hold,
            shown: 1,
            href: &back_to_next.jump_href,
            ready: true,
            reported: true,
            pinned: None,
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        let returned_pin = crate::visit::poll_chapter(crate::visit::ChapterPoll {
            saved_href: &returned_ready.saved_href,
            saved_page: returned_ready.saved_page,
            saved_offset: returned_ready.saved_offset,
            jump_href: &back_to_next.jump_href,
            jump_offset: returned_ready.jump_offset,
            awaiting: returned_ready.awaiting,
            pending: returned_ready.pending,
            hold: returned_ready.hold,
            shown: 2,
            href: &back_to_next.jump_href,
            ready: true,
            reported: true,
            pinned: Some(0),
            missed: false,
            sentence: 0,
            still_turning: false,
        });
        assert_eq!(returned.reveal_jump, None);
        assert_eq!(returned.saved_offset, 0);
        assert_eq!(returned_ready.reveal_jump, Some(0));
        assert_eq!(returned_ready.reveal_pending, None);
        assert_eq!(returned_pin.pin_write, true);
        assert_eq!(returned_pin.saved_offset, 0);
        assert_eq!(
            (
                resume.awaiting,
                resume.reveal_pending,
                resume_pin.saved_offset,
                back_resume.awaiting,
                back_resume.reveal_pending,
                back_pin.saved_offset,
            ),
            (Some(0), None, 0, Some(800), None, 800)
        );
        let back = crate::visit::opened_place(
            &spine,
            &resume_href(&file, path),
            resume_page(&file, path),
            resume_offset(&file, path),
        );
        assert_eq!(back.index, 1);
        assert_eq!(back.page, 1);
        assert_eq!(back.offset, 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_finished_swipe_is_the_place_a_later_open_reads() {
        let dir =
            std::env::temp_dir().join(format!("superreader-swipe-place-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        record_progress(&file, book, 1, None, 0, "ch.xhtml").unwrap();
        let mut reader = crate::visit::Reader::default();
        reader.place = crate::visit::ReadingPlace {
            href: "ch.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        reader.pending = 800;
        reader.awaiting = Some(800);
        reader.jump_href = "ch.xhtml".to_string();
        reader.jump_offset = Some(800);
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
        assert_eq!(
            reader.swipe.offer(down, 1, 10),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert_eq!(
            reader.swipe.offer(up, 1, 10),
            crate::visit::WheelMove::Commit(1)
        );
        let token = reader.swipe.commit_token().unwrap();
        reader.swipe.note_answer(token, "glide");
        let message = serde_json::json!({
            "page": 2,
            "pages": 10,
            "sentence": 50,
            "swipe": token,
            "missed": true,
            "pinned": 800,
            "kind": "box"
        });
        let mut store = SettingsBook { file: &file, book };
        let applied = crate::visit::handle_page(&mut reader, &message, "ch.xhtml", &mut store);
        assert!(applied.accepted);
        assert!(applied.reveal.is_none());
        assert_eq!(reader.place.page, 2);
        assert_eq!(reader.place.offset, 50);
        assert!(reader.pending == 0 && reader.awaiting.is_none() && reader.jump_offset.is_none());
        crate::visit::leave(&mut reader);
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(resume_page(&file, book), 2);
        assert_eq!(resume_offset(&file, book), 50);
        assert_eq!(reader.place.page, 2);
        assert_eq!(reader.place.offset, 50);
        assert_eq!(reader.pending, 50);
        assert!(
            reader.awaiting.is_none()
                && reader.jump_offset.is_none()
                && reader.hold.is_none()
                && reader.back.is_none()
        );

        let mut reader = crate::visit::Reader::default();
        reader.place.href = "ch.xhtml".to_string();
        reader.pending = 800;
        reader.awaiting = Some(800);
        reader.jump_href = "ch.xhtml".to_string();
        reader.jump_offset = Some(800);
        assert_eq!(
            reader.swipe.offer(down, 1, 10),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert_eq!(
            reader.swipe.offer(up, 1, 10),
            crate::visit::WheelMove::Commit(1)
        );
        let token = reader.swipe.commit_token().unwrap();
        reader.swipe.note_answer(token, "glide");
        let mut store = SettingsBook { file: &file, book };
        crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 2, "pages": 10, "sentence": 50, "swipe": token}),
            "ch.xhtml",
            &mut store,
        );
        let spine = vec!["ch.xhtml".to_string(), "next.xhtml".to_string()];
        let marks = vec![
            crate::visit::ChapterMark {
                title: "Here".to_string(),
                href: "ch.xhtml".to_string(),
                offset: 50,
                page: 2,
            },
            crate::visit::ChapterMark {
                title: "Next".to_string(),
                href: "next.xhtml".to_string(),
                offset: 0,
                page: 1,
            },
        ];
        let chapter =
            crate::visit::handle_chapter(&mut reader, 1, &marks, &spine, "ch.xhtml", &mut store);
        assert!(chapter.accepted);
        let back = reader.back.clone().expect("back");
        assert_eq!((back.page, back.offset), (2, 50));
        assert_eq!(
            reader.swipe.offer(down, 1, 10),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert_eq!(
            reader.swipe.offer(up, 1, 10),
            crate::visit::WheelMove::Commit(1)
        );
        assert!(reader.swipe.commit_token().is_some());
        crate::visit::leave(&mut reader);
        assert!(reader.swipe.commit_token().is_none());
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(resume_href(&file, book), "next.xhtml");
        assert_eq!(reader.place.page, 1);

        record_progress(&file, book, 1, None, 0, "ch.xhtml").unwrap();
        let mut reader = crate::visit::Reader::default();
        reader.place = crate::visit::ReadingPlace {
            href: "ch.xhtml".to_string(),
            page: 1,
            offset: 0,
        };
        reader.awaiting = Some(800);
        reader.hold = None;
        let mut store = SettingsBook { file: &file, book };
        let refused = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 4, "pages": 10, "sentence": 9, "swipe": 9, "pinned": 1, "missed": false}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!refused.accepted);
        assert_eq!(reader.place.offset, 0);
        assert_eq!(reader.awaiting, Some(800));
        assert_eq!(resume_offset(&file, book), 0);

        reader.awaiting = None;
        let late = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 2, "pages": 10, "pinned": 800, "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!late.wrote);
        assert_eq!(reader.place.page, 1);
        assert_eq!(resume_page(&file, book), 1);

        reader.pending = 800;
        reader.awaiting = None;
        reader.jump_offset = None;
        let sampled = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 3, "pages": 10, "sentence": 40, "frames": [8.0], "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!sampled.wrote);
        assert_eq!(reader.place.page, 1);
        assert_eq!(sampled.reveal, Some(800));
        reader.pending = 800;
        reader.awaiting = None;
        reader.reveal = None;
        let turned = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 3, "pages": 10, "sentence": 40, "kind": "turn", "frames": []}),
            "ch.xhtml",
            &mut store,
        );
        assert!(turned.wrote);
        assert!(turned.reveal.is_none());
        assert_eq!((reader.place.page, reader.place.offset), (3, 40));
        assert_eq!(reader.pending, 0);
        assert!(reader.awaiting.is_none() && reader.hold.is_none());

        let same = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 3, "pages": 10, "sentence": 0, "kind": "turn", "frames": []}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!same.wrote);
        assert_eq!(reader.place.offset, 40);

        reader.pending = 800;
        let boxed = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 9, "pages": 10, "sentence": 12, "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!boxed.wrote);
        assert_eq!(boxed.reveal, Some(800));
        assert_eq!(reader.place.page, 3);
        assert_eq!(resume_page(&file, book), 3);

        reader.place.page = 2;
        reader.place.offset = 10;
        reader.pending = 0;
        reader.awaiting = Some(800);
        reader.jump_offset = None;
        reader.reveal = None;
        let missed = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 2, "pages": 10, "sentence": 10, "pinned": 800, "missed": true, "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!missed.wrote);
        assert!(reader.awaiting.is_none());
        assert_eq!(reader.hold, Some(2));
        assert_eq!(reader.place.offset, 10);
        let later = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 2, "pages": 10, "sentence": 77, "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!later.wrote);
        assert_eq!(reader.place.offset, 10);
        assert_eq!(resume_offset(&file, book), 40);

        let held = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 8, "pages": 10, "sentence": 3, "hold": 800, "kind": "box"}),
            "ch.xhtml",
            &mut store,
        );
        assert!(!held.wrote);
        assert_eq!(reader.place.page, 2);

        let mut failed = reader.clone();
        failed.place.page = 1;
        failed.place.offset = 0;
        failed.swipe = crate::visit::Swipe::default();
        assert_eq!(
            failed.swipe.offer(down, 1, 10),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert_eq!(
            failed.swipe.offer(up, 1, 10),
            crate::visit::WheelMove::Commit(1)
        );
        let token = failed.swipe.commit_token().unwrap();
        failed.swipe.note_answer(token, "glide");
        let before = failed.place.clone();
        let mut empty = SettingsBook {
            file: &file,
            book: "",
        };
        let denied = crate::visit::handle_page(
            &mut failed,
            &serde_json::json!({"page": 2, "pages": 10, "sentence": 50, "swipe": token}),
            "ch.xhtml",
            &mut empty,
        );
        assert!(!denied.accepted);
        assert_eq!(failed.place, before);
        assert!(failed.swipe.commit_token().is_some());
        let mut zero = SettingsBook { file: &file, book };
        assert!(!zero.write_place(&crate::visit::ReadingPlace {
            href: "ch.xhtml".to_string(),
            page: 0,
            offset: 1,
        }));

        let mut reader = crate::visit::Reader::default();
        reader.place.href = "ch.xhtml".to_string();
        assert_eq!(
            reader.swipe.offer(down, 1, 10),
            crate::visit::WheelMove::Follow(80.0)
        );
        assert_eq!(
            reader.swipe.offer(up, 1, 10),
            crate::visit::WheelMove::Commit(1)
        );
        let token = reader.swipe.commit_token().unwrap();
        reader.swipe.note_answer(token, "glide");
        let dropped = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"swipe": token, "dropped": true, "page": 6, "pinned": 1}),
            "ch.xhtml",
            &mut store,
        );
        assert!(dropped.accepted);
        assert!(!dropped.paint);
        assert_eq!(reader.place.page, 1);
        assert!(reader.swipe.commit_token().is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_page_resume_does_not_store_the_columns_it_crosses() {
        let script = crate::reading::page_script("en");
        let dir =
            std::env::temp_dir().join(format!("superreader-page-walk-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let page_js = dir.join("page.js");
        let drive = dir.join("drive.js");
        std::fs::write(&page_js, script).unwrap();
        std::fs::write(&drive, PAGE_WALK_DRIVE).unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page_js)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let opened = read_posts(&mut stdout);
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        let mut reader = crate::visit::Reader::default();
        let mut store = SettingsBook { file: &file, book };
        for href in ["", "ch.xhtml"] {
            record_progress(&file, book, 10, None, 0, href).unwrap();
            crate::visit::open_resume(&mut reader, &store);
            let target = crate::visit::resume_walk(reader.place.page, reader.place.offset);
            assert_eq!(target, Some(10));
            assert_eq!(reader.pending, 0);
            let (shown, pages, _turning) =
                apply_posts(&mut reader, &opened, href, &mut store, 1, 1, false);
            assert_eq!(resume_page(&file, book), 10);
            assert_eq!(resume_offset(&file, book), 0);
            assert!(crate::visit::resume_turn(target, shown, pages, false));
            let (_, _, live) = read_frame(&mut stdin, &mut stdout, "now 0");
            let crossed_command = crate::visit::turn_command(
                1,
                crate::visit::TurnSource::Restore,
                reader.walk.unwrap_or(reader.place.page.max(1)),
                reader.restore_ack,
                reader.restore_load,
            );
            reader.restore_ack = false;
            let (crossed, crossed_answer, _) =
                read_frame(&mut stdin, &mut stdout, &crossed_command);
            if live < 10 {
                assert!(
                    crossed.iter().any(|post| post.get("frames").is_some()),
                    "{href} {crossed:?}"
                );
                assert!(crossed.iter().all(|post| post["kind"] != "turn"));
                apply_posts(&mut reader, &crossed, href, &mut store, shown, pages, true);
            } else {
                assert_eq!(
                    script_answer(&crossed_answer),
                    "no",
                    "{href} {crossed_answer}"
                );
                assert!(crossed.iter().all(|post| post["kind"] != "turn"));
            }
            assert_eq!(resume_page(&file, book), 10, "{href}");
            assert_eq!(resume_offset(&file, book), 0, "{href}");
            crate::visit::leave(&mut reader);
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!(reader.place.page, 10);
            assert_eq!(reader.place.offset, 0);
            assert_eq!(
                crate::visit::resume_walk(reader.place.page, reader.place.offset),
                Some(10)
            );

            record_progress(&file, book, 10, None, 0, href).unwrap();
            crate::visit::open_resume(&mut reader, &store);
            let mut shown = 1u32;
            let mut pages = 1u32;
            let mut turning = false;
            let (next_shown, next_pages, next_turning) = apply_posts(
                &mut reader,
                &opened,
                href,
                &mut store,
                shown,
                pages,
                turning,
            );
            shown = next_shown;
            pages = next_pages;
            turning = next_turning;
            let (_, _, live) = read_frame(&mut stdin, &mut stdout, "now 0");
            shown = live;
            let mut steps = 0u32;
            while crate::visit::resume_turn(crate::visit::resume_walk(10, 0), shown, pages, turning)
                && steps < 12
            {
                turning = true;
                let posts = read_step(
                    &mut stdin,
                    &mut stdout,
                    &crate::visit::turn_command(
                        1,
                        crate::visit::TurnSource::Restore,
                        reader.walk.unwrap_or(reader.place.page.max(1)),
                        reader.restore_ack,
                        reader.restore_load,
                    ),
                );
                let applied =
                    apply_posts(&mut reader, &posts, href, &mut store, shown, pages, turning);
                shown = applied.0;
                pages = applied.1;
                turning = applied.2;
                steps += 1;
                assert_eq!(resume_page(&file, book), 10, "{href}");
                assert_eq!(resume_offset(&file, book), 0, "{href}");
            }
            assert!(shown >= 10, "{href} stopped on {shown}");
            assert!(!crate::visit::resume_turn(Some(10), shown, pages, false));
            crate::visit::leave(&mut reader);
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!((reader.place.page, reader.place.offset), (10, 0));

            record_progress(&file, book, 10, None, 0, href).unwrap();
            crate::visit::open_resume(&mut reader, &store);
            let (shown, pages, turning) =
                apply_posts(&mut reader, &opened, href, &mut store, 1, 1, false);
            assert!(crate::visit::resume_turn(Some(10), shown, pages, turning));
            let chased = read_step(
                &mut stdin,
                &mut stdout,
                &crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::Restore,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            );
            let (shown, pages, turning) =
                apply_posts(&mut reader, &chased, href, &mut store, shown, pages, true);
            assert_eq!(resume_page(&file, book), 10);
            let taken = read_step(
                &mut stdin,
                &mut stdout,
                &crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            );
            assert!(taken.iter().any(|post| post["kind"] == "turn"));
            apply_posts(&mut reader, &taken, href, &mut store, shown, pages, turning);
            let saved = resume_page(&file, book);
            assert_ne!(saved, 10, "{href}");
            assert_eq!(saved, reader.place.page);
            crate::visit::leave(&mut reader);
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!(reader.place.page, saved);
            assert_eq!(resume_href(&file, book), href);
        }
        let _ = writeln_cmd(&mut stdin, "quit");
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_resume_walk_ends_when_the_reader_moves() {
        let script = crate::reading::page_script("en");
        let dir = std::env::temp_dir().join(format!("superreader-walk-end-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let page_js = dir.join("page.js");
        let drive = dir.join("drive.js");
        std::fs::write(&page_js, script).unwrap();
        std::fs::write(&drive, PAGE_WALK_DRIVE).unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page_js)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let _opened = read_posts(&mut stdout);
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        let mut reader = crate::visit::Reader::default();
        let mut store = SettingsBook { file: &file, book };
        for href in ["", "ch.xhtml"] {
            record_progress(&file, book, 10, None, 0, href).unwrap();
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!(reader.walk, Some(10));
            let mut shown = 1u32;
            let mut pages = 1u32;
            let polled = crate::visit::poll_resume(
                reader.walk,
                shown,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            reader.walk = polled.walk;
            let command = polled.command.expect("the walk still has columns");
            let crossed = read_step(&mut stdin, &mut stdout, &command);
            assert!(crossed.iter().any(|post| post["chase"] == true), "{href}");
            assert!(crossed.iter().all(|post| post["kind"] != "turn"), "{href}");
            let applied = apply_posts(&mut reader, &crossed, href, &mut store, shown, pages, true);
            shown = applied.0;
            pages = applied.1;
            assert_eq!(resume_page(&file, book), 10, "{href}");
            let taken = read_step(
                &mut stdin,
                &mut stdout,
                &crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            );
            assert!(
                taken.iter().any(|post| post["kind"] == "turn"),
                "{href} {taken:?}"
            );
            apply_posts(&mut reader, &taken, href, &mut store, shown, pages, false);
            let saved = resume_page(&file, book);
            assert_ne!(saved, 10, "{href}");
            assert_eq!(saved, reader.place.page, "{href}");
            assert!(reader.walk.is_none(), "{href}");
            shown = reader.place.page;
            for _ in 0..3 {
                let again = crate::visit::poll_resume(
                    reader.walk,
                    shown,
                    pages,
                    false,
                    reader.restore_ack,
                    reader.restore_load,
                );
                assert!(again.command.is_none(), "{href}");
                reader.walk = again.walk;
                assert_eq!(resume_page(&file, book), saved, "{href}");
            }
        }

        record_progress(&file, book, 10, None, 0, "").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        let mut shown = 1u32;
        let mut pages = 40u32;
        let mut left = false;
        let mut steps = 0u32;
        while steps < 12 {
            let polled = crate::visit::poll_resume(
                reader.walk,
                shown,
                pages,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            reader.walk = polled.walk;
            let Some(command) = polled.command else {
                break;
            };
            let posts = read_step(&mut stdin, &mut stdout, &command);
            let applied = apply_posts(&mut reader, &posts, "", &mut store, shown, pages, true);
            shown = applied.0;
            pages = applied.1;
            steps += 1;
            assert_eq!(resume_page(&file, book), 10);
            assert_eq!(resume_offset(&file, book), 0);
            if !left {
                crate::visit::leave(&mut reader);
                crate::visit::open_resume(&mut reader, &store);
                assert_eq!((reader.place.page, reader.place.offset), (10, 0));
                assert_eq!(reader.walk, Some(10));
                left = true;
            }
        }
        assert!(shown >= 10, "{shown}");
        assert!(reader.walk.is_none());
        let back = read_step(
            &mut stdin,
            &mut stdout,
            &crate::visit::turn_command(
                -1,
                crate::visit::TurnSource::User,
                reader.walk.unwrap_or(reader.place.page.max(1)),
                reader.restore_ack,
                reader.restore_load,
            ),
        );
        assert!(back.iter().any(|post| post["kind"] == "turn"), "{back:?}");
        apply_posts(&mut reader, &back, "", &mut store, shown, pages, false);
        let backed = resume_page(&file, book);
        assert_ne!(backed, 10);
        assert!(reader.walk.is_none());
        for _ in 0..3 {
            let again = crate::visit::poll_resume(
                reader.walk,
                backed,
                pages,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none());
            reader.walk = again.walk;
            assert_eq!(resume_page(&file, book), backed);
        }

        let mut store = SettingsBook { file: &file, book };
        for (label, command) in [
            (
                "left",
                crate::visit::turn_command(
                    crate::visit::page_key_delta(true, crate::visit::Surface::Reading, false),
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            (
                "right",
                crate::visit::turn_command(
                    crate::visit::page_key_delta(true, crate::visit::Surface::Reading, true),
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            (
                "space",
                crate::visit::turn_command(
                    crate::visit::page_key_delta(true, crate::visit::Surface::Reading, true),
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            (
                "shift-space",
                crate::visit::turn_command(
                    crate::visit::page_key_delta(true, crate::visit::Surface::Reading, false),
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            (
                "edge-back",
                crate::visit::turn_command(
                    -1,
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            (
                "edge-forward",
                crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            ("page-right", "keydown {\"key\":\"ArrowRight\"}".to_string()),
            ("page-left", "keydown {\"key\":\"ArrowLeft\"}".to_string()),
            ("page-space", "keydown {\"key\":\" \"}".to_string()),
            (
                "page-shift-space",
                "keydown {\"key\":\" \",\"shiftKey\":true}".to_string(),
            ),
            (
                "page-edge-forward",
                "click {\"clientX\":390,\"clientY\":20}".to_string(),
            ),
            (
                "page-edge-back",
                "click {\"clientX\":10,\"clientY\":20}".to_string(),
            ),
        ] {
            record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
            let mut reader = crate::visit::Reader::default();
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!(reader.walk, Some(10), "{label}");
            let posts = read_step(&mut stdin, &mut stdout, &command);
            assert!(
                posts.iter().any(|post| post["kind"] == "turn"),
                "{label} {posts:?}"
            );
            apply_posts(&mut reader, &posts, "ch.xhtml", &mut store, 1, 40, false);
            assert!(reader.walk.is_none(), "{label}");
            assert_eq!(resume_page(&file, book), reader.place.page, "{label}");
            let saved = reader.place.page;
            let again = crate::visit::poll_resume(
                reader.walk,
                3,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none(), "{label}");
            assert_eq!(resume_page(&file, book), saved, "{label}");
        }

        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut reader = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reader, &store);
        let (quiet, answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 11)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let parsed: serde_json::Value = serde_json::from_str(&answer).expect(&answer);
        reader
            .swipe
            .note_answer(11, parsed["answer"].as_str().unwrap());
        let (queued_posts, restore_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!(
                "now {}",
                crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::Restore,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load
                )
            ),
        );
        assert!(queued_posts.iter().all(|post| post["dropped"] != true));
        let refused = script_answer(&restore_answer);
        assert_eq!(reader.walk, Some(10), "{refused}");
        let finished = read_step(&mut stdin, &mut stdout, "void 0");
        assert!(finished.iter().all(|post| post["dropped"] != true));
        assert!(
            finished.iter().all(|post| post["chase"] != true),
            "{finished:?}"
        );
        let swipe_page = finished
            .iter()
            .find(|post| post["swipe"].as_u64() == Some(11))
            .unwrap()["page"]
            .as_u64()
            .unwrap() as u32;
        let screen = screen_page(&finished);
        apply_posts(&mut reader, &finished, "ch.xhtml", &mut store, 1, 40, false);
        assert_eq!(resume_page(&file, book), screen, "{refused} {finished:?}");
        assert_eq!(reader.place.page, screen);
        assert_eq!(swipe_page, screen);
        assert!(reader.walk.is_none());
        assert_eq!(refused, "no", "{restore_answer}");
        assert!(!crate::visit::turn_accepted(&refused));
        for _ in 0..3 {
            let again = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none());
            reader.walk = again.walk;
            assert_eq!(resume_page(&file, book), screen);
        }

        let (first_posts, first_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 21)),
        );
        assert!(first_posts.is_empty());
        let first: serde_json::Value = serde_json::from_str(&first_answer).unwrap();
        reader
            .swipe
            .note_answer(21, first["answer"].as_str().unwrap());
        let (second_posts, second_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 22)),
        );
        assert!(second_posts.iter().all(|post| post["dropped"] != true));
        let second: serde_json::Value = serde_json::from_str(&second_answer).unwrap();
        reader
            .swipe
            .note_answer(22, second["answer"].as_str().unwrap());
        let (chase_posts, restore_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!(
                "now {}",
                crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::Restore,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load
                )
            ),
        );
        assert_eq!(script_answer(&restore_answer), "no", "{restore_answer}");
        assert!(
            chase_posts.iter().all(|post| post["dropped"] != true),
            "{chase_posts:?}"
        );
        let released = read_step(&mut stdin, &mut stdout, "void 0");
        assert!(
            released.iter().all(|post| post["dropped"] != true),
            "{released:?}"
        );
        assert!(
            released.iter().all(|post| post["chase"] != true),
            "{released:?}"
        );
        assert!(
            released
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(22)),
            "{released:?}"
        );
        let screen = screen_page(&released);
        apply_posts(&mut reader, &released, "ch.xhtml", &mut store, 1, 40, false);
        assert_eq!(resume_page(&file, book), screen, "{released:?}");
        assert_eq!(reader.place.page, screen);

        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut reader = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reader, &store);
        crate::visit::stop_walk(&mut reader);
        assert!(crate::visit::poll_resume(
            reader.walk,
            3,
            40,
            false,
            reader.restore_ack,
            reader.restore_load
        )
        .command
        .is_none());
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);

        let _ = writeln_cmd(&mut stdin, "quit");
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_taken_turn_is_the_page_the_resume_stops_on() {
        let script = crate::reading::page_script("en");
        let dir = std::env::temp_dir().join(format!("superreader-taken-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let page_js = dir.join("page.js");
        let drive = dir.join("drive.js");
        std::fs::write(&page_js, script).unwrap();
        std::fs::write(&drive, PAGE_WALK_DRIVE).unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page_js)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let opened = read_posts(&mut stdout);
        let mut screen = opened
            .iter()
            .rev()
            .find_map(|post| post["page"].as_u64())
            .unwrap_or(1) as u32;
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut store = SettingsBook { file: &file, book };
        let mut reader = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(reader.walk, Some(10));
        let mut steps = 0u32;
        while steps < 12 {
            let polled = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            reader.walk = polled.walk;
            let Some(command) = polled.command else {
                break;
            };
            let posts = read_step(&mut stdin, &mut stdout, &command);
            assert!(posts.iter().all(|post| post["kind"] != "turn"), "{posts:?}");
            apply_posts(
                &mut reader,
                &posts,
                "ch.xhtml",
                &mut store,
                screen,
                40,
                true,
            );
            screen = screen_page(&posts);
            steps += 1;
            assert_eq!(resume_page(&file, book), 10);
            assert_eq!(resume_offset(&file, book), 0);
        }
        assert_eq!(screen, 10, "{steps}");
        assert!(reader.walk.is_none());
        assert_eq!(resume_page(&file, book), screen);

        record_progress(&file, book, 20, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(reader.walk, Some(20));
        let before = screen;
        let (quiet, release_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 11)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let release: serde_json::Value =
            serde_json::from_str(&release_answer).expect(&release_answer);
        reader
            .swipe
            .note_answer(11, release["answer"].as_str().unwrap());
        let polled = crate::visit::poll_resume(
            reader.walk,
            before,
            40,
            false,
            reader.restore_ack,
            reader.restore_load,
        );
        reader.walk = polled.walk;
        let command = polled.command.expect("the commit has not cleared the walk");
        let (mid, restore_answer) =
            read_answered(&mut stdin, &mut stdout, &format!("now {command}"));
        assert!(mid.is_empty(), "{mid:?}");
        let refused = script_answer(&restore_answer);
        assert_eq!(reader.walk, Some(20), "{refused}");
        let finished = read_step(&mut stdin, &mut stdout, "void 0");
        screen = screen_page(&finished);
        apply_posts(
            &mut reader,
            &finished,
            "ch.xhtml",
            &mut store,
            before,
            40,
            false,
        );
        let saved = resume_page(&file, book);
        assert!(
            saved == screen && refused == "no" && finished.iter().all(|post| post["chase"] != true),
            "saved {saved} screen {screen} answer {refused} posts {finished:?}"
        );
        assert_eq!(reader.place.page, screen);
        assert!(reader.walk.is_none());
        assert!(!crate::visit::turn_accepted(&refused));
        for _ in 0..3 {
            let again = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none());
            reader.walk = again.walk;
            assert_eq!(resume_page(&file, book), saved);
        }

        record_progress(&file, book, 30, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(reader.walk, Some(30));
        let before = screen;
        for token in [21u64, 22] {
            let (posts, answer) = read_answered(
                &mut stdin,
                &mut stdout,
                &format!("now {}", crate::visit::release_command(1, token)),
            );
            assert!(
                posts.iter().all(|post| post["dropped"] != true),
                "{posts:?}"
            );
            let parsed: serde_json::Value = serde_json::from_str(&answer).expect(&answer);
            reader
                .swipe
                .note_answer(token, parsed["answer"].as_str().unwrap());
        }
        let polled = crate::visit::poll_resume(
            reader.walk,
            before,
            40,
            false,
            reader.restore_ack,
            reader.restore_load,
        );
        reader.walk = polled.walk;
        let command = polled
            .command
            .expect("two queued commits still leave the walk");
        let (mid, restore_answer) =
            read_answered(&mut stdin, &mut stdout, &format!("now {command}"));
        let refused = script_answer(&restore_answer);
        assert_eq!(refused, "no", "{restore_answer}");
        assert!(!crate::visit::turn_accepted(&refused));
        assert_eq!(reader.walk, Some(30));
        assert!(mid.iter().all(|post| post["dropped"] != true), "{mid:?}");
        let released = read_step(&mut stdin, &mut stdout, "void 0");
        assert!(
            released.iter().all(|post| post["dropped"] != true),
            "{released:?}"
        );
        assert!(
            released.iter().all(|post| post["chase"] != true),
            "{released:?}"
        );
        assert!(
            released
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(22)),
            "{released:?}"
        );
        screen = screen_page(&released);
        apply_posts(
            &mut reader,
            &released,
            "ch.xhtml",
            &mut store,
            before,
            40,
            false,
        );
        assert_eq!(screen, before + 2, "{released:?}");
        assert_eq!(resume_page(&file, book), screen);
        assert_eq!(reader.place.page, screen);
        assert!(reader.walk.is_none());
        for _ in 0..3 {
            let again = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none());
            reader.walk = again.walk;
            assert_eq!(resume_page(&file, book), screen);
        }

        for (label, command) in [
            (
                "shell",
                crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::User,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load,
                ),
            ),
            ("page-key", "keydown {\"key\":\"ArrowRight\"}".to_string()),
        ] {
            record_progress(&file, book, 40, None, 0, "ch.xhtml").unwrap();
            crate::visit::open_resume(&mut reader, &store);
            assert_eq!(reader.walk, Some(40), "{label}");
            let before = screen;
            let started = read_step(
                &mut stdin,
                &mut stdout,
                &format!(
                    "now {}",
                    crate::visit::turn_command(
                        1,
                        crate::visit::TurnSource::Restore,
                        reader.walk.unwrap_or(reader.place.page.max(1)),
                        reader.restore_ack,
                        reader.restore_load
                    )
                ),
            );
            assert!(started.is_empty(), "{label} {started:?}");
            assert_eq!(reader.walk, Some(40), "{label}");
            let posts = read_step(&mut stdin, &mut stdout, &command);
            assert!(
                posts.iter().all(|post| post["chase"] != true),
                "{label} {posts:?}"
            );
            assert!(
                posts.iter().any(|post| post["kind"] == "turn"),
                "{label} {posts:?}"
            );
            screen = screen_page(&posts);
            apply_posts(
                &mut reader,
                &posts,
                "ch.xhtml",
                &mut store,
                before,
                40,
                false,
            );
            assert_eq!(screen, before + 1, "{label} {posts:?}");
            assert_eq!(resume_page(&file, book), screen, "{label}");
            assert_eq!(reader.place.page, screen, "{label}");
            assert!(reader.walk.is_none(), "{label}");
            let again = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none(), "{label}");
        }

        record_progress(&file, book, 40, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        let before = screen;
        let started = read_step(
            &mut stdin,
            &mut stdout,
            &format!(
                "now {}",
                crate::visit::turn_command(
                    1,
                    crate::visit::TurnSource::Restore,
                    reader.walk.unwrap_or(reader.place.page.max(1)),
                    reader.restore_ack,
                    reader.restore_load
                )
            ),
        );
        assert!(started.is_empty(), "{started:?}");
        let (quiet, swipe_answer) = read_answered(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 31)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let swipe: serde_json::Value = serde_json::from_str(&swipe_answer).expect(&swipe_answer);
        reader
            .swipe
            .note_answer(31, swipe["answer"].as_str().unwrap());
        let posts = read_step(&mut stdin, &mut stdout, "void 0");
        assert!(posts.iter().all(|post| post["chase"] != true), "{posts:?}");
        screen = screen_page(&posts);
        apply_posts(
            &mut reader,
            &posts,
            "ch.xhtml",
            &mut store,
            before,
            40,
            false,
        );
        assert_eq!(screen, before + 1, "{posts:?}");
        assert_eq!(resume_page(&file, book), screen);
        assert_eq!(reader.place.page, screen);
        assert!(reader.walk.is_none());
        assert!(crate::visit::poll_resume(
            reader.walk,
            screen,
            40,
            false,
            reader.restore_ack,
            reader.restore_load
        )
        .command
        .is_none());

        record_progress(&file, book, 40, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        let kept = resume_page(&file, book);
        let (_, _, before) = read_frame(&mut stdin, &mut stdout, "now 0");
        let issued = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            reader.walk.unwrap_or(reader.place.page.max(1)),
            reader.restore_ack,
            reader.restore_load,
        );
        let (started, answer, sought) =
            read_frame(&mut stdin, &mut stdout, &format!("now {issued}"));
        assert!(started.is_empty(), "{started:?}");
        assert_eq!(script_answer(&answer), "ok", "{answer}");
        assert_eq!(sought, before + 1, "{sought}");
        assert_eq!(
            crate::visit::place_step(true, true, false),
            crate::visit::PlaceStep::Cancel { reflow: false }
        );
        assert_eq!(reader.walk, Some(kept));
        let command = crate::visit::cancel_placed_page(&mut reader);
        let (dropped, _, _column) = read_frame(&mut stdin, &mut stdout, &format!("now {command}"));
        assert!(
            dropped.iter().all(|post| post["chase"] != true),
            "{dropped:?}"
        );
        assert_eq!(reader.walk, Some(kept));
        assert_eq!(resume_page(&file, book), kept);
        assert_eq!(resume_offset(&file, book), 0);
        let polled = crate::visit::poll_resume(
            reader.walk,
            before,
            40,
            false,
            reader.restore_ack,
            reader.restore_load,
        );
        let again = polled
            .command
            .expect("a resize leaves the saved page to walk");
        reader.walk = polled.walk;
        let (posts, _, column) = read_frame(&mut stdin, &mut stdout, &again);
        assert!(posts.iter().any(|post| post["chase"] == true), "{posts:?}");
        apply_posts(
            &mut reader,
            &posts,
            "ch.xhtml",
            &mut store,
            column,
            40,
            true,
        );
        assert_eq!(resume_page(&file, book), kept);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(reader.walk, Some(kept));

        let _ = writeln_cmd(&mut stdin, "quit");
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_resize_keeps_the_saved_page_until_the_anchor_lands() {
        let script = crate::reading::page_script("en");
        let dir = std::env::temp_dir().join(format!("superreader-cancel-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let page_js = dir.join("page.js");
        let drive = dir.join("drive.js");
        std::fs::write(&page_js, script).unwrap();
        std::fs::write(&drive, PAGE_WALK_DRIVE).unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page_js)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let _opened = read_posts(&mut stdout);
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut store = SettingsBook { file: &file, book };
        let mut reader = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reader, &store);
        let mut screen = 1u32;
        while screen < 4 {
            let polled = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            reader.walk = polled.walk;
            let command = polled.command.expect("walk");
            let (posts, _, column) = read_frame(&mut stdin, &mut stdout, &command);
            assert!(posts.iter().all(|post| post["kind"] != "turn"), "{posts:?}");
            apply_posts(
                &mut reader,
                &posts,
                "ch.xhtml",
                &mut store,
                screen,
                40,
                true,
            );
            screen = column;
            assert_eq!(resume_page(&file, book), 10);
            assert_eq!(resume_offset(&file, book), 0);
        }
        assert_eq!(screen, 4);
        assert_eq!(reader.walk, Some(10));
        let issued = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            reader.walk.unwrap_or(reader.place.page.max(1)),
            reader.restore_ack,
            reader.restore_load,
        );
        let (mid, answer, sought) = read_frame(&mut stdin, &mut stdout, &format!("now {issued}"));
        assert!(mid.is_empty(), "{mid:?}");
        assert_eq!(script_answer(&answer), "ok");
        assert_eq!(sought, 5);
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(
            crate::visit::place_step(false, true, true),
            crate::visit::PlaceStep::Cancel { reflow: true }
        );
        let command = crate::visit::cancel_placed_page(&mut reader);
        let (dropped, _, _) = read_frame(&mut stdin, &mut stdout, &format!("now {command}"));
        assert!(
            dropped.iter().all(|post| post["chase"] != true),
            "{dropped:?}"
        );
        assert_eq!(reader.walk, Some(10));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        let (settled_posts, _, settled_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(settled_posts.is_empty(), "{settled_posts:?}");
        assert_eq!(settled_column, 4);
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            r#"window.__caretOff=null;window.__textNode={nodeType:3,textContent:"a".repeat(5000),nodeValue:"a".repeat(5000),parentNode:document.body,parentElement:document.body};document.body.firstChild=window.__textNode;document.caretRangeFromPoint=function(){if(window.__caretOff==null)return null;return{startContainer:window.__textNode,startOffset:window.__caretOff}};document.createTreeWalker=function(){var sent=false;return{currentNode:window.__textNode,nextNode:function(){if(sent)return null;sent=true;this.currentNode=window.__textNode;return window.__textNode}}};document.createRange=function(){return{setStart:function(){},setEnd:function(){},selectNodeContents:function(){},getClientRects:function(){return[]},getBoundingClientRect:function(){return{left:900,right:910,top:40,bottom:60,width:10,height:20}}}};"#,
        );
        let mut tick = fitted_tick(false, false);
        let measured = crate::visit::layout_frame(&mut reader, tick);
        let crate::visit::Draw::Measure { page: 10, probe } = measured.draw else {
            panic!("measure the saved column, {measured:?}");
        };
        assert!(measured.cancel);
        assert!(measured.pump);
        tick.awaiting = true;
        tick.hold_probe = probe;
        let pending = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(pending.draw, crate::visit::Draw::Rest);
        assert!(!pending.cancel);
        assert!(pending.pump);
        assert_eq!(reader.probe, probe);
        let (missed_posts, missed, missed_column) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(10, probe)),
        );
        assert!(missed.is_empty(), "{missed:?} {missed_posts:?}");
        assert!(
            missed_posts.iter().all(|post| post.get("hold").is_none()),
            "{missed_posts:?}"
        );
        assert!(
            missed_posts.iter().any(|post| post["missed"] == true),
            "{missed_posts:?}"
        );
        assert!(
            missed_posts
                .iter()
                .all(|post| post["pinned"] != 3600 && post["hold"] != 3600),
            "{missed_posts:?}"
        );
        apply_posts(
            &mut reader,
            &missed_posts,
            "ch.xhtml",
            &mut store,
            missed_column,
            40,
            true,
        );
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(reader.walk, Some(10));
        assert!(reader.measured.is_none());
        assert!(!reader.anchoring);
        assert!(reader.hold_layout);
        tick.missed = true;
        let stayed = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(stayed.draw, crate::visit::Draw::Move { reveal: None });
        assert!(!stayed.cancel);
        assert!(!stayed.pump);
        assert_ne!(reader.probe, probe);
        assert_eq!(reader.walk, Some(10));
        tick.awaiting = false;
        tick.missed = false;
        tick.fitted_same = true;
        let mut rested_cancels = 0;
        let mut rested_pumps = 0;
        for _ in 0..8 {
            let rested = crate::visit::layout_frame(&mut reader, tick);
            assert_eq!(rested.draw, crate::visit::Draw::Rest);
            if rested.cancel {
                rested_cancels += 1;
            }
            if rested.pump {
                rested_pumps += 1;
            }
            assert_eq!(reader.walk, Some(10));
        }
        assert_eq!(rested_cancels, 0);
        assert_eq!(rested_pumps, 0);
        let blocked = crate::visit::poll_anchored(&mut reader, 4, 40, false);
        assert!(blocked.command.is_none());
        assert_eq!(reader.walk, Some(10));
        let reached = crate::visit::poll_anchored(&mut reader, 10, 40, false);
        assert!(reached.command.is_none());
        assert_eq!(reader.walk, Some(10));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(crate::visit::current_hold(Some(42), probe, reader.probe).is_none());
        let late = serde_json::json!({
            "hold": 42,
            "measured": true,
            "probe": probe,
            "page": 4,
            "pages": 40,
            "goal": 10,
        });
        apply_posts(&mut reader, &[late], "ch.xhtml", &mut store, 4, 40, true);
        assert!(reader.measured.is_none());
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(reader.walk, Some(10));
        let user = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::User,
            0,
            false,
            reader.restore_load,
        );
        let (user_posts, user_answer, user_column) =
            read_frame(&mut stdin, &mut stdout, &format!("now {user}"));
        assert!(user_posts.is_empty(), "{user_posts:?}");
        assert_eq!(script_answer(&user_answer), "ok");
        assert_eq!(user_column, 5);
        let drop_user = crate::visit::cancel_placed_page(&mut reader);
        let (user_dropped, _, user_back) =
            read_frame(&mut stdin, &mut stdout, &format!("now {drop_user}"));
        assert!(
            user_dropped.iter().all(|post| post["kind"] != "turn"),
            "{user_dropped:?}"
        );
        assert_eq!(user_back, 4);
        assert_eq!(reader.walk, Some(10));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        let still = crate::visit::poll_anchored(&mut reader, 4, 40, false);
        assert!(still.command.is_none());
        assert_eq!(reader.walk, Some(10));
        let (quiet, open_answer, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 7)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let open_release: serde_json::Value =
            serde_json::from_str(&open_answer).expect(&open_answer);
        assert_eq!(open_release["answer"].as_str(), Some("glide"));
        reader.swipe.note_answer(7, "glide");
        let drop_open = crate::visit::cancel_placed_page(&mut reader);
        let (open_dropped, _, open_column) =
            read_frame(&mut stdin, &mut stdout, &format!("now {drop_open}"));
        assert!(
            open_dropped
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(7) && post["dropped"] == true),
            "{open_dropped:?}"
        );
        let (open_late, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            open_late
                .iter()
                .all(|post| post["swipe"].as_u64() != Some(7)),
            "{open_late:?}"
        );
        apply_posts(
            &mut reader,
            &open_dropped,
            "ch.xhtml",
            &mut store,
            open_column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(reader.walk, Some(10));
        tick.fitted_same = false;
        tick.awaiting = false;
        tick.missed = false;
        let again = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(
            again.draw,
            crate::visit::Draw::Move { reveal: None },
            "{again:?}"
        );
        assert!(!again.cancel);
        assert!(!again.pump);
        assert!(reader.measured.is_none());
        assert_eq!(reader.walk, Some(10));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);

        let (quiet, release_answer, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 11)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let release: serde_json::Value =
            serde_json::from_str(&release_answer).expect(&release_answer);
        reader
            .swipe
            .note_answer(11, release["answer"].as_str().unwrap());
        let issued = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            reader.walk.unwrap_or(reader.place.page.max(1)),
            reader.restore_ack,
            reader.restore_load,
        );
        let (finished, _, column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            finished
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(11)),
            "{finished:?}"
        );
        let cancel = crate::visit::cancel_placed_page(&mut reader);
        let (during, _, kept_column) =
            read_frame(&mut stdin, &mut stdout, &format!("now {cancel}"));
        assert!(
            during.iter().all(|post| post["swipe"].as_u64() != Some(11)),
            "{during:?}"
        );
        assert_eq!(kept_column, column);
        apply_posts(&mut reader, &finished, "ch.xhtml", &mut store, 1, 40, false);
        let saved = resume_page(&file, book);
        assert_eq!(column, saved, "{finished:?}");
        assert_eq!(reader.place.page, saved);
        assert!(reader.walk.is_none());
        let (late_posts, late_answer, late_column) =
            read_frame(&mut stdin, &mut stdout, &format!("now {issued}"));
        assert_eq!(script_answer(&late_answer), "no", "{late_answer}");
        assert!(
            late_posts.iter().all(|post| post["chase"] != true),
            "{late_posts:?}"
        );
        assert_eq!(late_column, saved);
        let (tail, _, tail_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(tail.iter().all(|post| post["chase"] != true), "{tail:?}");
        assert_eq!(tail_column, saved);
        for _ in 0..3 {
            let again = crate::visit::poll_resume(
                reader.walk,
                saved,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            assert!(again.command.is_none());
            reader.walk = again.walk;
            assert_eq!(resume_page(&file, book), saved);
        }

        record_progress(&file, book, saved + 2, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        let mut screen = saved;
        let target = saved + 2;
        loop {
            let polled = crate::visit::poll_resume(
                reader.walk,
                screen,
                40,
                false,
                reader.restore_ack,
                reader.restore_load,
            );
            reader.walk = polled.walk;
            let Some(command) = polled.command else {
                break;
            };
            let (posts, _, column) = read_frame(&mut stdin, &mut stdout, &command);
            assert!(posts.iter().any(|post| post["chase"] == true), "{posts:?}");
            apply_posts(
                &mut reader,
                &posts,
                "ch.xhtml",
                &mut store,
                screen,
                40,
                true,
            );
            screen = column;
            assert_eq!(resume_page(&file, book), target);
        }
        assert_eq!(screen, target);
        assert!(reader.walk.is_none());

        record_progress(&file, book, 30, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(reader.walk, Some(30));
        let clearing = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            30,
            reader.restore_ack,
            reader.restore_load,
        );
        reader.restore_ack = false;
        let (cleared, clear_answer, _) = read_frame(&mut stdin, &mut stdout, &clearing);
        assert_eq!(script_answer(&clear_answer), "ok", "{clear_answer}");
        apply_posts(
            &mut reader,
            &cleared,
            "ch.xhtml",
            &mut store,
            screen,
            40,
            true,
        );
        assert_eq!(resume_page(&file, book), 30);
        assert_eq!(resume_offset(&file, book), 0);
        let stale = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            30,
            false,
            reader.restore_load,
        );
        let (quiet, refused_answer, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 41)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let refused_release: serde_json::Value =
            serde_json::from_str(&refused_answer).expect(&refused_answer);
        assert_eq!(refused_release["answer"].as_str(), Some("glide"));
        let (refused_posts, _, refused_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            refused_posts
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(41)),
            "{refused_posts:?}"
        );
        let (stale_posts, stale_answer, stale_column) =
            read_frame(&mut stdin, &mut stdout, &format!("now {stale}"));
        assert_eq!(script_answer(&stale_answer), "no", "{stale_answer}");
        assert!(stale_posts.is_empty(), "{stale_posts:?}");
        assert_eq!(stale_column, refused_column);
        let place_before = reader.place.clone();
        let walk_before = reader.walk;
        apply_posts(
            &mut reader,
            &refused_posts,
            "ch.xhtml",
            &mut store,
            refused_column,
            40,
            false,
        );
        assert_eq!(reader.place, place_before);
        assert_eq!(reader.walk, walk_before);
        assert!(reader.restore_ack);
        assert_eq!(resume_page(&file, book), 30);
        let resumed = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            30,
            reader.restore_ack,
            reader.restore_load,
        );
        reader.restore_ack = false;
        let (resumed_posts, resumed_answer, resumed_column) =
            read_frame(&mut stdin, &mut stdout, &resumed);
        assert_eq!(script_answer(&resumed_answer), "ok", "{resumed_answer}");
        assert!(
            resumed_posts.iter().any(|post| post["chase"] == true),
            "{resumed_posts:?}"
        );
        assert_eq!(resumed_column, refused_column + 1);
        apply_posts(
            &mut reader,
            &resumed_posts,
            "ch.xhtml",
            &mut store,
            resumed_column,
            40,
            true,
        );
        assert_eq!(resume_page(&file, book), 30);
        assert_eq!(resume_offset(&file, book), 0);

        let (quiet, fail_answer, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 42)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let fail_release: serde_json::Value =
            serde_json::from_str(&fail_answer).expect(&fail_answer);
        reader
            .swipe
            .note_answer(42, fail_release["answer"].as_str().unwrap());
        let place_before = reader.place.clone();
        let walk_before = reader.walk;
        let (failed_posts, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        struct ClosedFile(crate::visit::ReadingPlace);
        impl crate::visit::PlaceFile for ClosedFile {
            fn write_place(&mut self, _: &crate::visit::ReadingPlace) -> bool {
                false
            }
            fn resume(&self) -> crate::visit::ReadingPlace {
                self.0.clone()
            }
        }
        let mut closed = ClosedFile(place_before.clone());
        for post in &failed_posts {
            crate::visit::handle_page(&mut reader, post, "ch.xhtml", &mut closed);
        }
        assert_eq!(reader.place, place_before);
        assert_eq!(reader.walk, walk_before);
        assert!(reader.restore_ack);
        assert_eq!(resume_page(&file, book), 30);
        let after_fail = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::Restore,
            30,
            reader.restore_ack,
            reader.restore_load,
        );
        reader.restore_ack = false;
        let (after_posts, after_answer, _) = read_frame(&mut stdin, &mut stdout, &after_fail);
        assert_eq!(script_answer(&after_answer), "ok", "{after_answer}");
        assert!(
            after_posts.iter().any(|post| post["chase"] == true),
            "{after_posts:?}"
        );
        apply_posts(
            &mut reader,
            &after_posts,
            "ch.xhtml",
            &mut store,
            resumed_column,
            40,
            true,
        );
        assert_eq!(resume_page(&file, book), 30);
        assert_eq!(resume_offset(&file, book), 0);
        assert_eq!(reader.place.page, 30);

        assert_eq!(reader.walk, Some(30));
        let (quiet, done_answer, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::release_command(1, 43)),
        );
        assert!(quiet.is_empty(), "{quiet:?}");
        let done_release: serde_json::Value =
            serde_json::from_str(&done_answer).expect(&done_answer);
        reader
            .swipe
            .note_answer(43, done_release["answer"].as_str().unwrap());
        let (done_posts, _, done_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            done_posts
                .iter()
                .any(|post| post["swipe"].as_u64() == Some(43)),
            "{done_posts:?}"
        );
        assert_eq!(reader.walk, Some(30));
        let cancel = crate::visit::cancel_placed_page(&mut reader);
        let (cancel_posts, _, _) = read_frame(&mut stdin, &mut stdout, &format!("now {cancel}"));
        assert!(
            cancel_posts
                .iter()
                .all(|post| post["swipe"].as_u64() != Some(43)),
            "{cancel_posts:?}"
        );
        assert_eq!(reader.walk, Some(30));
        apply_posts(
            &mut reader,
            &done_posts,
            "ch.xhtml",
            &mut store,
            done_column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), done_column);
        assert_eq!(reader.place.page, done_column);
        assert!(reader.walk.is_none());

        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        assert!(!resume_caret(&file, book));
        let mut reopened = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reopened, &store);
        assert_eq!(reopened.walk, Some(10));
        assert!(reopened.measured.is_none());
        let zero_measure = crate::visit::layout_frame(&mut reopened, fitted_tick(false, false));
        let crate::visit::Draw::Measure { page: 10, .. } = zero_measure.draw else {
            panic!("an old page row is measured, {zero_measure:?}");
        };
        assert!(reopened.measured.is_none());
        let _ = read_frame(&mut stdin, &mut stdout, &one_line(COLUMN_CHARS));
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "document.body.scrollLeft=0;window.innerWidth=400;window.__caretHit=false;window.__lineTop=40;window.__lineBottom=60;window.__text=\"a\".repeat(800);window.__images=[];window.__dead=function(){return false}",
        );
        record_progress(&file, book, 1, None, 0, "ch.xhtml").unwrap();
        let mut origin = crate::visit::Reader::default();
        crate::visit::open_resume(&mut origin, &store);
        assert!(origin.walk.is_none());
        let origin_measure = crate::visit::layout_frame(&mut origin, fitted_tick(false, false));
        let crate::visit::Draw::Measure { page: 0, probe } = origin_measure.draw else {
            panic!("page 1 reads the visible column, {origin_measure:?}");
        };
        let (zero_posts, zero, zero_visible) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(0, probe)),
        );
        assert_eq!(zero, "0", "{zero_posts:?}");
        assert_eq!(zero_visible, 1, "{zero_posts:?}");
        apply_posts(
            &mut origin,
            &zero_posts,
            "ch.xhtml",
            &mut store,
            1,
            40,
            true,
        );
        assert_eq!(origin.measured, Some(0));
        assert_eq!(resume_page(&file, book), 1);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(!resume_caret(&file, book));
        let mut zero_tick = fitted_tick(false, true);
        zero_tick.hold = Some(0);
        zero_tick.hold_probe = probe;
        let zero_plan = crate::visit::layout_frame(&mut origin, zero_tick);
        assert_eq!(zero_plan.draw, crate::visit::Draw::Move { reveal: Some(0) });
        assert!(origin.anchoring);
        let (zero_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(0)",
        );
        assert!(zero_wait.is_empty(), "{zero_wait:?}");
        let (zero_landed, _, zero_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert_eq!(zero_column, 1, "{zero_landed:?}");
        assert!(
            zero_landed
                .iter()
                .any(|post| post["pinned"] == 0 && post["kind"] == "anchor"),
            "{zero_landed:?}"
        );
        apply_posts(
            &mut origin,
            &zero_landed,
            "ch.xhtml",
            &mut store,
            zero_column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), zero_column);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(resume_caret(&file, book));
        let mut opened = crate::visit::Reader::default();
        crate::visit::open_resume(&mut opened, &store);
        assert_eq!(opened.measured, Some(0));
        assert!(opened.walk.is_none());
        let kept_zero = crate::visit::layout_frame(&mut opened, fitted_tick(false, false));
        assert_eq!(kept_zero.draw, crate::visit::Draw::Move { reveal: Some(0) });
        assert!(!kept_zero.pump);
        let mut quiet = fitted_tick(true, false);
        for _ in 0..5 {
            let rested = crate::visit::layout_frame(&mut opened, quiet);
            assert_eq!(rested.draw, crate::visit::Draw::Rest);
            assert!(!rested.cancel);
            assert!(!rested.pump);
        }
        let user = crate::visit::turn_command(
            1,
            crate::visit::TurnSource::User,
            0,
            false,
            opened.restore_load,
        );
        let (started_posts, started_answer, _) =
            read_frame(&mut stdin, &mut stdout, &format!("now {user}"));
        assert!(started_posts.is_empty(), "{started_posts:?}");
        assert_eq!(script_answer(&started_answer), "ok");
        for _ in 0..5 {
            let during = crate::visit::layout_frame(&mut opened, quiet);
            assert!(!during.cancel, "{during:?}");
            assert!(!during.pump, "{during:?}");
        }
        let (finished_turn, _, finished_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            finished_turn.iter().any(|post| post["kind"] == "turn"),
            "{finished_turn:?}"
        );
        apply_posts(
            &mut opened,
            &finished_turn,
            "ch.xhtml",
            &mut store,
            finished_column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), finished_column);
        assert!(opened.walk.is_none());
        quiet.shown = finished_column;
        let after = crate::visit::layout_frame(&mut opened, quiet);
        assert!(!after.pump);
        assert!(!after.cancel);

        let _ = writeln_cmd(&mut stdin, "quit");
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_hold_reads_the_column_character() {
        let dir = std::env::temp_dir().join(format!("superreader-column-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let page_js = dir.join("page.js");
        let drive = dir.join("drive.js");
        std::fs::write(&page_js, crate::reading::page_script("en")).unwrap();
        std::fs::write(&drive, PAGE_WALK_DRIVE).unwrap();
        let mut child = std::process::Command::new("node")
            .arg(&drive)
            .arg(&page_js)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("node");
        let mut stdin = child.stdin.take().unwrap();
        let mut stdout = std::io::BufReader::new(child.stdout.take().unwrap());
        let _opened = read_posts(&mut stdout);
        let _ = read_frame(&mut stdin, &mut stdout, &one_line(COLUMN_CHARS));
        let file = dir.join("settings.json");
        let book = dir.join("book.epub");
        let book = book.to_str().unwrap();
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut store = SettingsBook { file: &file, book };
        let mut reader = crate::visit::Reader::default();
        crate::visit::open_resume(&mut reader, &store);
        assert_eq!(reader.walk, Some(10));
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "document.body.scrollLeft=1200;window.innerWidth=400;window.innerHeight=800;window.__caretHit=false;window.__lineTop=40;window.__lineBottom=60;window.__text=\"a\".repeat(5000);window.__images=[];window.__dead=function(){return false}",
        );
        let mut tick = fitted_tick(false, false);
        let measured = crate::visit::layout_frame(&mut reader, tick);
        let crate::visit::Draw::Measure { page: 10, probe } = measured.draw else {
            panic!("measure the saved column, {measured:?}");
        };
        let (held_posts, held, held_column) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(10, probe)),
        );
        assert_eq!(held, "360", "{held_posts:?}");
        assert_eq!(held_column, 4, "{held_posts:?}");
        assert!(
            held_posts
                .iter()
                .any(|post| post["measured"] == true && post["hold"] == 360),
            "{held_posts:?}"
        );
        apply_posts(
            &mut reader,
            &held_posts,
            "ch.xhtml",
            &mut store,
            4,
            40,
            true,
        );
        assert_eq!(reader.measured, Some(360));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        tick.awaiting = true;
        tick.hold = Some(360);
        tick.hold_probe = probe;
        let reveal = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(reveal.draw, crate::visit::Draw::Move { reveal: Some(360) });
        assert!(!reveal.cancel);
        assert!(reader.anchoring);
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__caretHit=true;window.__lineTop=100;window.__lineBottom=220",
        );
        let (hit_posts, hit, hit_column) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeHold&&window.spikeHold(10,9)",
        );
        assert_eq!(hit, "368", "{hit_posts:?}");
        assert_ne!(hit, "360");
        assert_eq!(hit_column, 4, "{hit_posts:?}");
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__caretHit=false;window.__lineTop=40;window.__lineBottom=60;window.__text=\" \".repeat(363)+\"x\".repeat(2000)",
        );
        let (gap_posts, gap, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeHold&&window.spikeHold(10,9)",
        );
        assert_eq!(gap, "363", "{gap_posts:?}");
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__text=\"a\".repeat(5000);window.__dead=function(i){return i<360}",
        );
        let (dead_posts, dead, dead_column) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeHold&&window.spikeHold(10,9)",
        );
        assert_eq!(dead, "360", "{dead_posts:?}");
        assert_eq!(dead_column, 4, "{dead_posts:?}");
        let _ = read_frame(&mut stdin, &mut stdout, "window.innerWidth=250");
        let (waiting, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(360)",
        );
        assert!(waiting.is_empty(), "{waiting:?}");
        let (landed, _, column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert_eq!(column, 15, "{landed:?}");
        assert!(
            landed
                .iter()
                .any(|post| post["kind"] == "anchor" && post["pinned"] == 360),
            "{landed:?}"
        );
        assert!(
            landed
                .iter()
                .all(|post| post["pinned"] != 225 && post["hold"] != 225),
            "{landed:?}"
        );
        let mut denied = SettingsBook {
            file: &file,
            book: "",
        };
        for post in &landed {
            crate::visit::handle_page(&mut reader, post, "ch.xhtml", &mut denied);
        }
        assert!(reader.restore_ack);
        assert_eq!(reader.measured, Some(360));
        assert_eq!(reader.walk, Some(10));
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(crate::visit::poll_anchored(&mut reader, 15, 40, false)
            .command
            .is_none());
        tick.awaiting = false;
        tick.hold = None;
        tick.fitted_same = false;
        let denied_resize = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(
            denied_resize.draw,
            crate::visit::Draw::Move { reveal: Some(360) }
        );
        assert!(!denied_resize.cancel);
        apply_posts(
            &mut reader,
            &landed,
            "ch.xhtml",
            &mut store,
            column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), 15);
        assert_eq!(resume_offset(&file, book), 360);
        assert!(resume_caret(&file, book));
        assert!(reader.walk.is_none());
        let _ = read_frame(&mut stdin, &mut stdout, "window.innerWidth=180");
        tick.fitted_same = false;
        let narrower = crate::visit::layout_frame(&mut reader, tick);
        assert_eq!(
            narrower.draw,
            crate::visit::Draw::Move { reveal: Some(360) }
        );
        assert!(!narrower.cancel);
        let (second_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(360)",
        );
        assert!(second_wait.is_empty(), "{second_wait:?}");
        let (second, _, second_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert_eq!(second_column, 21, "{second:?}");
        assert!(
            second.iter().any(|post| post["pinned"] == 360),
            "{second:?}"
        );
        apply_posts(
            &mut reader,
            &second,
            "ch.xhtml",
            &mut store,
            second_column,
            40,
            false,
        );
        assert_eq!(resume_page(&file, book), 21);
        assert_eq!(resume_offset(&file, book), 360);
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(360)",
        );
        let _ = read_frame(&mut stdin, &mut stdout, "frame");
        let user = crate::visit::turn_command(1, crate::visit::TurnSource::User, 0, false, 0);
        let (during, during_answer, _) =
            read_frame(&mut stdin, &mut stdout, &format!("now {user}"));
        assert!(during.is_empty(), "{during:?}");
        assert_eq!(script_answer(&during_answer), "ok");
        let (after_turn, _, _) = read_frame(&mut stdin, &mut stdout, "frame");
        assert!(
            after_turn.iter().all(|post| post.get("pinned").is_none()),
            "{after_turn:?}"
        );
        let (turned, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            turned.iter().any(|post| post["kind"] == "turn"),
            "{turned:?}"
        );
        assert!(
            turned.iter().all(|post| post.get("pinned").is_none()),
            "{turned:?}"
        );
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.spikeJustify={whenVisibleCommitted:function(){return{then:function(fn){window.__go=fn}}}}",
        );
        let (armed, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(999999)",
        );
        assert!(armed.is_empty(), "{armed:?}");
        let (preempted, _, _) = read_frame(&mut stdin, &mut stdout, &format!("now {user}"));
        assert!(preempted.is_empty(), "{preempted:?}");
        let (late, _, _) = read_frame(&mut stdin, &mut stdout, "now window.__go&&window.__go()");
        assert!(
            late.iter().all(|post| post.get("pinned").is_none()),
            "{late:?}"
        );
        let (late_turn, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert!(
            late_turn.iter().all(|post| post.get("pinned").is_none()),
            "{late_turn:?}"
        );
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.spikeJustify=null;window.innerWidth=250",
        );
        let (kept_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeReveal&&window.spikeReveal(360)",
        );
        assert!(kept_wait.is_empty(), "{kept_wait:?}");
        let _ = read_frame(&mut stdin, &mut stdout, "now window.innerWidth=250");
        let (kept_land, _, kept_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert_eq!(kept_column, 15, "{kept_land:?}");
        assert!(
            kept_land.iter().any(|post| post["pinned"] == 360),
            "{kept_land:?}"
        );
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let mut pictures = crate::visit::Reader::default();
        crate::visit::open_resume(&mut pictures, &store);
        assert_eq!(pictures.walk, Some(10));
        assert!(!resume_caret(&file, book));
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "document.body.scrollLeft=1200;window.innerWidth=400;window.__text=\"\";window.__caretHit=false;window.__imageAt=3610;window.__images=[{tagName:\"IMG\",id:\"pic\",closest:function(){return null},getBoundingClientRect:function(){var left=(window.__imageAt||0)-(document.body.scrollLeft||0);return{left:left,right:left+80,top:40,bottom:200,width:80,height:160}}}]",
        );
        let mut picture_tick = fitted_tick(false, false);
        let picture_measure = crate::visit::layout_frame(&mut pictures, picture_tick);
        let crate::visit::Draw::Measure { page: 10, probe } = picture_measure.draw else {
            panic!("an image column is still sampled once, {picture_measure:?}");
        };
        let (picture_posts, picture, picture_column) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(10, probe)),
        );
        assert!(picture.is_empty(), "{picture:?} {picture_posts:?}");
        assert_eq!(picture_column, 4, "{picture_posts:?}");
        assert!(
            picture_posts.iter().any(|post| post["figure"] == "img#pic"),
            "{picture_posts:?}"
        );
        assert!(
            picture_posts.iter().all(|post| post.get("hold").is_none()),
            "{picture_posts:?}"
        );
        let figure_post = picture_posts
            .iter()
            .find(|post| post["figure"] == "img#pic")
            .expect("img post");
        assert!(figure_post.get("page").is_some(), "{figure_post:?}");
        assert_eq!(
            figure_post["probe"],
            serde_json::json!(pictures.probe),
            "{figure_post:?}"
        );
        let mut kept = None;
        let mut lost = false;
        let applied = route_figure(
            &mut pictures,
            &mut kept,
            &mut lost,
            figure_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!applied.paint);
        assert!(!applied.wrote);
        assert_eq!(kept.as_deref(), Some("img#pic"));
        assert!(!lost);
        assert_eq!(pictures.figure.as_deref(), Some("img#pic"));
        assert!(pictures.measured.is_none());
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(!resume_caret(&file, book));
        let (saw, missed_figure) = crate::visit::figure_flags(&kept, lost);
        picture_tick.awaiting = true;
        picture_tick.figure = saw;
        picture_tick.figure_lost = missed_figure;
        let aligned = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(aligned.draw, crate::visit::Draw::Align);
        assert!(!aligned.cancel);
        let mut late = figure_post.clone();
        late["probe"] = serde_json::json!(pictures.probe.saturating_add(9));
        late["figure"] = serde_json::json!("svg@9");
        crate::visit::keep_message_figure(
            &mut kept,
            &mut lost,
            &late,
            pictures.probe,
            pictures.figure.as_deref(),
        );
        assert_eq!(kept.as_deref(), Some("img#pic"), "{late:?}");
        let mut bare = figure_post.clone();
        bare.as_object_mut().unwrap().remove("page");
        bare["figure"] = serde_json::json!("svg@9");
        crate::visit::keep_message_figure(
            &mut kept,
            &mut lost,
            &bare,
            pictures.probe,
            pictures.figure.as_deref(),
        );
        assert_eq!(kept.as_deref(), Some("img#pic"), "{bare:?}");
        let loaded_reader = pictures.clone();
        let loaded_mail = kept.clone();
        let taken = pictures.clone();
        let taken_mail = kept.clone();
        let mut opened = loaded_reader;
        let mut opened_mail = loaded_mail;
        let mut opened_lost = false;
        crate::visit::open_resume(&mut opened, &store);
        crate::visit::drop_figure_mail(&mut opened_mail, &mut opened_lost);
        assert!(opened.figure.is_none());
        assert!(opened_mail.is_none());
        picture_tick.figure = false;
        picture_tick.figure_lost = false;
        picture_tick.awaiting = false;
        picture_tick.fitted_same = false;
        let opened_draw = crate::visit::layout_frame(&mut opened, picture_tick);
        assert_ne!(
            opened_draw.draw,
            crate::visit::Draw::Align,
            "{opened_draw:?}"
        );
        let mut chapter = pictures.clone();
        let mut chapter_mail = kept.clone();
        let mut chapter_lost = false;
        crate::visit::reset_restore_epoch(&mut chapter);
        crate::visit::drop_figure_mail(&mut chapter_mail, &mut chapter_lost);
        assert!(chapter.figure.is_none());
        assert!(chapter_mail.is_none());
        let chapter_draw = crate::visit::layout_frame(&mut chapter, picture_tick);
        assert_ne!(
            chapter_draw.draw,
            crate::visit::Draw::Align,
            "{chapter_draw:?}"
        );
        let _ = read_frame(&mut stdin, &mut stdout, "window.innerWidth=250");
        picture_tick.awaiting = false;
        picture_tick.figure = false;
        picture_tick.figure_lost = false;
        picture_tick.fitted_same = false;
        let again = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(again.draw, crate::visit::Draw::Align, "{again:?}");
        let (align_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeAlign&&window.spikeAlign(\"img#pic\")",
        );
        assert!(align_wait.is_empty(), "{align_wait:?}");
        let (aligned_posts, _, aligned_column) = read_frame(&mut stdin, &mut stdout, "void 0");
        assert_eq!(aligned_column, 15, "{aligned_posts:?}");
        let aligned_post = aligned_posts
            .iter()
            .find(|post| post["figure"] == "img#pic")
            .expect("align post");
        assert!(aligned_post.get("probe").is_none(), "{aligned_post:?}");
        let aligned_apply = route_figure(
            &mut pictures,
            &mut kept,
            &mut lost,
            aligned_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!aligned_apply.paint);
        assert!(!aligned_apply.wrote);
        assert_eq!(pictures.figure.as_deref(), Some("img#pic"));
        assert!(!lost);
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(!resume_caret(&file, book));
        assert!(pictures.measured.is_none());
        let svg_reader = pictures.clone();
        let (first_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.__images=[];window.spikeAlign&&window.spikeAlign(\"img#pic\")",
        );
        assert!(first_wait.is_empty(), "{first_wait:?}");
        let (first_miss, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        let first_post = first_miss
            .iter()
            .find(|post| post["figure"] == "img#pic" && post["missed"] == true)
            .expect("first miss");
        assert!(first_post.get("page").is_some(), "{first_post:?}");
        let first_apply = route_figure(
            &mut pictures,
            &mut kept,
            &mut lost,
            first_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!first_apply.paint);
        assert!(!first_apply.wrote);
        assert!(lost);
        assert_eq!(
            crate::visit::found_figure(kept.clone(), lost),
            None,
            "a loss is not noted back onto the reader"
        );
        let (retry_saw, retry_lost) = crate::visit::figure_flags(&kept, lost);
        assert!(
            !picture_tick.awaiting,
            "the first align already ended the wait"
        );
        picture_tick.figure = retry_saw;
        picture_tick.figure_lost = retry_lost;
        picture_tick.fitted_same = true;
        let retry = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(retry.draw, crate::visit::Draw::Align, "{retry:?}");
        assert_eq!(pictures.figure_misses, 1, "{retry:?}");
        assert_eq!(pictures.figure.as_deref(), Some("img#pic"));
        assert!(crate::visit::spent_figure_loss(retry_lost, retry.draw));
        crate::visit::drop_figure_mail(&mut kept, &mut lost);
        picture_tick.figure = false;
        picture_tick.figure_lost = false;
        picture_tick.fitted_same = false;
        let still = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(still.draw, crate::visit::Draw::Align, "{still:?}");
        assert_eq!(pictures.figure_misses, 1);
        let mut recovered = pictures.clone();
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__images=[{tagName:\"IMG\",id:\"pic\",closest:function(){return null},getBoundingClientRect:function(){var left=3610-(document.body.scrollLeft||0);return{left:left,right:left+80,top:40,bottom:200,width:80,height:160}}}];window.innerWidth=400",
        );
        let (found_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeAlign&&window.spikeAlign(\"img#pic\")",
        );
        assert!(found_wait.is_empty(), "{found_wait:?}");
        let (found_posts, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        let found_post = found_posts
            .iter()
            .find(|post| post["figure"] == "img#pic" && post.get("missed").is_none())
            .expect("found figure");
        assert!(found_post.get("probe").is_none(), "{found_post:?}");
        let found_saved = found_post.clone();
        let mut found_mail = None;
        let mut found_lost = false;
        let mut wrong = found_saved.clone();
        wrong["probe"] = serde_json::json!(recovered.probe.saturating_add(9));
        crate::visit::keep_message_figure(
            &mut found_mail,
            &mut found_lost,
            &wrong,
            recovered.probe,
            recovered.figure.as_deref(),
        );
        assert!(found_mail.is_none(), "{wrong:?}");
        assert_eq!(recovered.figure_misses, 1);
        let found_apply = route_figure(
            &mut recovered,
            &mut found_mail,
            &mut found_lost,
            &found_saved,
            "ch.xhtml",
            &mut store,
        );
        assert!(!found_apply.paint);
        assert!(!found_apply.wrote);
        assert_eq!(found_mail.as_deref(), Some("img#pic"));
        assert!(!found_lost);
        let (found_saw, found_lost_flag) = crate::visit::figure_flags(&found_mail, found_lost);
        let mut found_tick = picture_tick;
        assert!(!found_tick.awaiting);
        found_tick.figure = found_saw;
        found_tick.figure_lost = found_lost_flag;
        found_tick.fitted_same = true;
        let found_draw = crate::visit::layout_frame(&mut recovered, found_tick);
        assert_eq!(found_draw.draw, crate::visit::Draw::Align, "{found_draw:?}");
        assert_eq!(recovered.figure_misses, 0);
        assert_eq!(recovered.figure.as_deref(), Some("img#pic"));
        assert!(crate::visit::spent_figure_find(found_saw, found_draw.draw));
        crate::visit::drop_figure_mail(&mut found_mail, &mut found_lost);
        found_tick.figure = false;
        found_tick.figure_lost = false;
        let found_again = crate::visit::layout_frame(&mut recovered, found_tick);
        assert_eq!(
            found_again.draw,
            crate::visit::Draw::Rest,
            "{found_again:?}"
        );
        assert_eq!(recovered.figure_misses, 0);
        let one_apply = route_figure(
            &mut recovered,
            &mut found_mail,
            &mut found_lost,
            first_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!one_apply.wrote);
        assert!(found_lost);
        let (one_saw, one_lost) = crate::visit::figure_flags(&found_mail, found_lost);
        found_tick.figure = one_saw;
        found_tick.figure_lost = one_lost;
        found_tick.fitted_same = false;
        let one = crate::visit::layout_frame(&mut recovered, found_tick);
        assert_eq!(one.draw, crate::visit::Draw::Align, "{one:?}");
        assert_eq!(recovered.figure_misses, 1);
        assert_eq!(recovered.figure.as_deref(), Some("img#pic"));
        assert!(crate::visit::spent_figure_loss(one_lost, one.draw));
        crate::visit::drop_figure_mail(&mut found_mail, &mut found_lost);
        found_tick.figure = false;
        found_tick.figure_lost = false;
        let one_again = crate::visit::layout_frame(&mut recovered, found_tick);
        assert_eq!(one_again.draw, crate::visit::Draw::Align, "{one_again:?}");
        assert_eq!(recovered.figure_misses, 1);
        let last_apply = route_figure(
            &mut recovered,
            &mut found_mail,
            &mut found_lost,
            first_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!last_apply.wrote);
        let (last_saw, last_lost) = crate::visit::figure_flags(&found_mail, found_lost);
        found_tick.figure = last_saw;
        found_tick.figure_lost = last_lost;
        let last = crate::visit::layout_frame(&mut recovered, found_tick);
        assert_eq!(
            last.draw,
            crate::visit::Draw::Move { reveal: None },
            "{last:?}"
        );
        assert!(recovered.figure.is_none());
        assert_eq!(recovered.figure_misses, 2);
        assert_eq!(recovered.sample_misses, 2);
        crate::visit::drop_figure_mail(&mut found_mail, &mut found_lost);
        let ghost = route_figure(
            &mut recovered,
            &mut found_mail,
            &mut found_lost,
            &found_saved,
            "ch.xhtml",
            &mut store,
        );
        assert!(!ghost.wrote);
        assert!(found_mail.is_none());
        assert!(recovered.figure.is_none(), "{recovered:?}");
        let (second_wait, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            "now window.spikeAlign&&window.spikeAlign(\"img#pic\")",
        );
        assert!(second_wait.is_empty(), "{second_wait:?}");
        let (second_miss, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        let second_post = second_miss
            .iter()
            .find(|post| post["figure"] == "img#pic" && post["missed"] == true)
            .expect("second miss");
        let second_apply = route_figure(
            &mut pictures,
            &mut kept,
            &mut lost,
            second_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(!second_apply.wrote);
        assert!(picture_tick.fitted_same == false && !picture_tick.awaiting);
        let (give_saw, give_lost) = crate::visit::figure_flags(&kept, lost);
        picture_tick.figure = give_saw;
        picture_tick.figure_lost = give_lost;
        let gave_up = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(
            gave_up.draw,
            crate::visit::Draw::Move { reveal: None },
            "{gave_up:?}"
        );
        assert!(pictures.figure.is_none());
        assert_eq!(pictures.figure_misses, 2);
        assert_eq!(pictures.sample_misses, 2);
        assert!(crate::visit::spent_figure_loss(give_lost, gave_up.draw));
        crate::visit::drop_figure_mail(&mut kept, &mut lost);
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        assert!(!resume_caret(&file, book));
        picture_tick.figure = false;
        picture_tick.figure_lost = false;
        let skipped_figure = crate::visit::layout_frame(&mut pictures, picture_tick);
        assert_eq!(
            skipped_figure.draw,
            crate::visit::Draw::Move { reveal: None },
            "{skipped_figure:?}"
        );
        assert!(pictures.figure.is_none(), "a spent loss is not noted again");
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__images=[{tagName:\"SVG\",id:\"\",closest:function(){return null},getBoundingClientRect:function(){var left=3610-(document.body.scrollLeft||0);return{left:left,right:left+40,top:40,bottom:200,width:40,height:160}}}];window.innerWidth=400",
        );
        let (indexed_posts, _, _) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(10, svg_reader.probe)),
        );
        let indexed = indexed_posts
            .iter()
            .find(|post| post["figure"] == "svg@0")
            .expect("svg post");
        let mut svg_kept = None;
        let mut svg_lost = false;
        let mut svg_reader = svg_reader;
        let indexed_apply = route_figure(
            &mut svg_reader,
            &mut svg_kept,
            &mut svg_lost,
            indexed,
            "ch.xhtml",
            &mut store,
        );
        assert!(!indexed_apply.paint);
        assert!(!indexed_apply.wrote);
        assert_eq!(svg_kept.as_deref(), Some("svg@0"));
        assert!(!svg_lost);
        let (svg_saw, svg_lost_flag) = crate::visit::figure_flags(&svg_kept, svg_lost);
        let mut svg_tick = fitted_tick(false, false);
        svg_tick.figure = svg_saw;
        svg_tick.figure_lost = svg_lost_flag;
        let svg_draw = crate::visit::layout_frame(&mut svg_reader, svg_tick);
        assert_eq!(svg_draw.draw, crate::visit::Draw::Align, "{svg_draw:?}");
        assert_eq!(svg_reader.figure.as_deref(), Some("svg@0"));
        assert!(!resume_caret(&file, book));
        let mut turned = taken;
        let mut turned_mail = taken_mail;
        let mut turned_lost = false;
        let (turn_now, _, _) = read_frame(&mut stdin, &mut stdout, &format!("now {user}"));
        let (turn_later, _, _) = read_frame(&mut stdin, &mut stdout, "void 0");
        let turn_post = turn_now
            .iter()
            .chain(turn_later.iter())
            .find(|post| post["kind"] == "turn")
            .expect("turn post");
        let _ = route_figure(
            &mut turned,
            &mut turned_mail,
            &mut turned_lost,
            turn_post,
            "ch.xhtml",
            &mut store,
        );
        assert!(turned.figure.is_none(), "{turned:?}");
        assert!(turned_mail.is_none());
        let _ = route_figure(
            &mut turned,
            &mut turned_mail,
            &mut turned_lost,
            &found_saved,
            "ch.xhtml",
            &mut store,
        );
        assert!(turned.figure.is_none(), "{turned:?}");
        assert!(turned_mail.is_none());
        assert!(!resume_caret(&file, book));
        picture_tick.figure = false;
        picture_tick.figure_lost = false;
        picture_tick.awaiting = false;
        picture_tick.fitted_same = false;
        let turned_draw = crate::visit::layout_frame(&mut turned, picture_tick);
        assert_ne!(
            turned_draw.draw,
            crate::visit::Draw::Align,
            "{turned_draw:?}"
        );
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        let _ = read_frame(
            &mut stdin,
            &mut stdout,
            "window.__text=\"\";window.__images=[];window.innerWidth=400;document.body.scrollLeft=1200",
        );
        let mut blank = crate::visit::Reader::default();
        record_progress(&file, book, 10, None, 0, "ch.xhtml").unwrap();
        crate::visit::open_resume(&mut blank, &store);
        let mut blank_tick = fitted_tick(false, false);
        let blank_measure = crate::visit::layout_frame(&mut blank, blank_tick);
        let crate::visit::Draw::Measure { page: 10, probe } = blank_measure.draw else {
            panic!("a blank column is sampled on the old frame, {blank_measure:?}");
        };
        let (blank_posts, blank_answer, blank_column) = read_frame(
            &mut stdin,
            &mut stdout,
            &format!("now {}", crate::visit::hold_eval(10, probe)),
        );
        assert!(blank_answer.is_empty(), "{blank_answer:?} {blank_posts:?}");
        assert_eq!(blank_column, 4, "{blank_posts:?}");
        assert!(
            blank_posts.iter().any(|post| post["missed"] == true),
            "{blank_posts:?}"
        );
        blank_tick.awaiting = true;
        blank_tick.hold_probe = probe;
        blank_tick.missed = true;
        let lost = crate::visit::layout_frame(&mut blank, blank_tick);
        assert_eq!(lost.draw, crate::visit::Draw::Move { reveal: None });
        assert_eq!(blank.sample_misses, 2);
        assert!(blank.measured.is_none());
        blank_tick.awaiting = false;
        blank_tick.missed = false;
        blank_tick.fitted_same = false;
        let skipped = crate::visit::layout_frame(&mut blank, blank_tick);
        assert_eq!(
            skipped.draw,
            crate::visit::Draw::Move { reveal: None },
            "{skipped:?}"
        );
        assert_eq!(resume_page(&file, book), 10);
        assert_eq!(resume_offset(&file, book), 0);
        let _ = writeln_cmd(&mut stdin, "quit");
        let _ = child.wait();
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn fitted_tick(same: bool, awaiting: bool) -> crate::visit::FrameTick {
        let mut tick = crate::visit::FrameTick::new(same, true, true, true, true, awaiting);
        tick.shown = 4;
        tick.pages = 40;
        tick
    }

    fn route_figure(
        reader: &mut crate::visit::Reader,
        kept: &mut Option<String>,
        lost: &mut bool,
        post: &serde_json::Value,
        href: &str,
        store: &mut SettingsBook<'_>,
    ) -> crate::visit::Apply {
        let applied = crate::visit::handle_page(reader, post, href, store);
        let known = reader.figure.clone();
        crate::visit::keep_message_figure(kept, lost, post, reader.probe, known.as_deref());
        crate::visit::remember_figure(reader, crate::visit::found_figure(kept.clone(), *lost));
        applied
    }

    fn apply_posts(
        reader: &mut crate::visit::Reader,
        posts: &[serde_json::Value],
        href: &str,
        store: &mut SettingsBook<'_>,
        mut shown: u32,
        mut pages: u32,
        mut turning: bool,
    ) -> (u32, u32, bool) {
        for post in posts {
            crate::visit::handle_page(reader, post, href, store);
            if post.get("frames").is_some() {
                turning = false;
            }
            if let Some(page) = post["page"].as_u64() {
                shown = page as u32;
            }
            if let Some(count) = post["pages"].as_u64() {
                pages = count as u32;
            }
        }
        (shown, pages, turning)
    }

    fn read_posts(stdout: &mut impl std::io::BufRead) -> Vec<serde_json::Value> {
        let mut line = String::new();
        stdout.read_line(&mut line).expect("node");
        let value: serde_json::Value = serde_json::from_str(&line).expect(&line);
        value["posts"].as_array().cloned().unwrap_or_default()
    }

    fn read_step(
        stdin: &mut impl std::io::Write,
        stdout: &mut impl std::io::BufRead,
        command: &str,
    ) -> Vec<serde_json::Value> {
        read_answered(stdin, stdout, command).0
    }

    fn read_answered(
        stdin: &mut impl std::io::Write,
        stdout: &mut impl std::io::BufRead,
        command: &str,
    ) -> (Vec<serde_json::Value>, String) {
        writeln_cmd(stdin, command);
        let mut line = String::new();
        stdout.read_line(&mut line).expect("node");
        let value: serde_json::Value = serde_json::from_str(&line).expect(&line);
        let posts = value["posts"].as_array().cloned().unwrap_or_default();
        let answer = value["answer"].as_str().unwrap_or("").to_string();
        (posts, answer)
    }

    fn writeln_cmd(stdin: &mut impl std::io::Write, command: &str) {
        stdin.write_all(command.as_bytes()).unwrap();
        stdin.write_all(b"\n").unwrap();
        stdin.flush().unwrap();
    }

    fn screen_page(posts: &[serde_json::Value]) -> u32 {
        posts
            .iter()
            .rev()
            .find_map(|post| post["page"].as_u64())
            .expect("screen") as u32
    }

    fn one_line(script: &str) -> String {
        script.replace(['\n', '\r'], " ")
    }

    fn read_frame(
        stdin: &mut impl std::io::Write,
        stdout: &mut impl std::io::BufRead,
        command: &str,
    ) -> (Vec<serde_json::Value>, String, u32) {
        writeln_cmd(stdin, command);
        let mut line = String::new();
        stdout.read_line(&mut line).expect("node");
        let value: serde_json::Value = serde_json::from_str(&line).expect(&line);
        let posts = value["posts"].as_array().cloned().unwrap_or_default();
        let answer = value["answer"].as_str().unwrap_or("").to_string();
        let column = value["column"].as_u64().expect("column") as u32;
        (posts, answer, column)
    }

    fn script_answer(raw: &str) -> String {
        serde_json::from_str::<serde_json::Value>(raw)
            .ok()
            .and_then(|value| value["answer"].as_str().map(str::to_string))
            .unwrap_or_else(|| raw.to_string())
    }
}

const COLUMN_CHARS: &str = r#"
if (window.__text == null) window.__text = "a".repeat(5000);
if (!window.__dead) window.__dead = function () { return false; };
if (!window.__images) window.__images = [];
function columnNode() {
  var text = window.__text || "";
  var n = window.__textNode;
  if (!n) {
    n = { nodeType: 3, parentNode: document.body, parentElement: document.body };
    window.__textNode = n;
  }
  n.textContent = text;
  n.nodeValue = text;
  document.body.firstChild = n;
  document.body.textContent = text;
  return n;
}
document.createTreeWalker = function () {
  var sent = false;
  return { currentNode: columnNode(), nextNode: function () {
    var n = columnNode();
    if (sent || !(n.textContent || "").length) return null;
    sent = true;
    this.currentNode = n;
    return n;
  } };
};
document.createRange = function () {
  var start = 0;
  return {
    setStart: function (_n, offset) { start = offset; },
    setEnd: function () {},
    selectNodeContents: function () { start = 0; },
    getClientRects: function () {
      var rect = this.getBoundingClientRect();
      return rect.width ? [rect] : [];
    },
    getBoundingClientRect: function () {
      if (window.__dead(start)) return { left: 0, right: 0, top: 0, bottom: 0, width: 0, height: 0 };
      var left = start * 10 - (document.body.scrollLeft || 0);
      var top = window.__lineTop == null ? 40 : window.__lineTop;
      var bottom = window.__lineBottom == null ? 60 : window.__lineBottom;
      return { left: left, right: left + 10, top: top, bottom: bottom, width: 10, height: 20 };
    }
  };
};
document.caretRangeFromPoint = function (x, y) {
  if (!window.__caretHit) return null;
  var top = window.__lineTop == null ? 40 : window.__lineTop;
  var bottom = window.__lineBottom == null ? 60 : window.__lineBottom;
  if (y < top || y >= bottom) return null;
  var n = columnNode();
  var index = Math.floor(((document.body.scrollLeft || 0) + x) / 10);
  if (index < 0 || index >= (n.textContent || "").length) return null;
  if (window.__dead(index)) return null;
  return { startContainer: n, startOffset: index };
};
document.querySelectorAll = function (sel) {
  var images = window.__images || [];
  if (sel === "img, svg") return images.slice();
  if (sel === "img") return images.filter(function (el) { return String(el.tagName).toLowerCase() === "img"; });
  if (sel === "svg") return images.filter(function (el) { return String(el.tagName).toLowerCase() === "svg"; });
  return [];
};
columnNode();
"#;

const PAGE_WALK_DRIVE: &str = r#"
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
global.addEventListener = (type, fn) => {
  (listeners[type] ||= []).push(fn);
};
global.dispatchEvent = (event) => {
  fire(event && event.type, event || {});
};
function fire(type, event) {
  for (const fn of listeners[type] || []) fn(event);
}
global.window = global;
window.innerWidth = 400;
window.innerHeight = 800;
window.webkit = { messageHandlers: { spike: { postMessage(raw){ posts.push(JSON.parse(raw)); } } } };
function flush() {
  global.__now += 1000;
  const batch = raf.splice(0, raf.length);
  for (const fn of batch) fn(global.__now);
}
function emit(value) {
  const w = Math.max(1, Math.round(window.innerWidth));
  value.column = Math.round((body.scrollLeft || 0) / w) + 1;
  value.scroll = body.scrollLeft || 0;
  value.width = w;
  fs.writeSync(1, JSON.stringify(value) + "\n");
}
eval(script);
flush();
flush();
emit({ posts: posts.slice() });
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });
function drain() {
  for (let i = 0; i < 8 && raf.length; i++) flush();
}
rl.on('line', (line) => {
  if (line === 'quit') process.exit(0);
  posts.length = 0;
  let answer = null;
  if (line.startsWith('keydown ')) {
    fire('keydown', Object.assign({ repeat: false, shiftKey: false, preventDefault() {} }, JSON.parse(line.slice(8))));
  } else if (line === 'frame') {
    global.__now += 1000;
    const fn = raf.shift();
    if (fn) fn(global.__now);
  } else if (line.startsWith('click ')) {
    const spec = JSON.parse(line.slice(6));
    fire('mousedown', { button: 0, clientX: spec.clientX, clientY: spec.clientY || 0 });
    fire('click', Object.assign({ button: 0, preventDefault() {} }, spec));
  } else {
    const defer = line.startsWith('now ');
    answer = eval(defer ? line.slice(4) : line);
  }
  if (!line.startsWith('now ') && line !== 'frame') drain();
  emit({ posts: posts.slice(), answer: answer == null ? null : String(answer) });
});
"#;
