pub use spike_bold_word_starts::decorate;
use spike_bold_word_starts::BOLD_WORD_STARTS_LABEL;

pub fn menu_label() -> String {
    format!(
        "{} · {}",
        BOLD_WORD_STARTS_LABEL.0, BOLD_WORD_STARTS_LABEL.1
    )
}
