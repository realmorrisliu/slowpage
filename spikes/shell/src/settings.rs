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
        let _ = std::fs::remove_dir_all(&dir);
    }
}
