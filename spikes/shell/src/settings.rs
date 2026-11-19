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
    write_history(path, book, page, total, Some(opened_at), None, None)
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
    write_history(path, book, page, total, None, Some(offset), Some(href))
}

fn write_history(
    path: &Path,
    book: &str,
    page: u32,
    total: Option<u32>,
    opened_at: Option<i64>,
    offset: Option<u32>,
    href: Option<&str>,
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
        let turned = crate::visit::handle_page(
            &mut reader,
            &serde_json::json!({"page": 3, "pages": 10, "sentence": 40, "frames": [8.0], "kind": "box"}),
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
}
