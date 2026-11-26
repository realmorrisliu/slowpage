use superreader_typeset::{ChapterInput, Typesetter, PAPERBACK_HEIGHT, PAPERBACK_WIDTH};

fn fonts() -> Option<Typesetter> {
    let dir = "/tmp/superreader-fixtures";
    let names = [
        "EBGaramond-Regular.ttf",
        "EBGaramond-Italic.ttf",
        "NotoSerifCJKsc-Regular.otf",
    ];
    let missing: Vec<&str> = names
        .into_iter()
        .filter(|name| !std::path::Path::new(dir).join(name).is_file())
        .collect();
    if !missing.is_empty() {
        let message = format!(
            "fixture fonts are not in {dir} ({}); run just fixtures-fonts",
            missing.join(", ")
        );
        if std::env::var_os("CI").is_some() {
            panic!("{message}");
        }
        let name = std::thread::current()
            .name()
            .unwrap_or("reading")
            .to_string();
        let line = format!("skip {name}: {message}\n");
        if let Ok(mut err) = std::fs::OpenOptions::new().write(true).open("/dev/stderr") {
            let _ = std::io::Write::write_all(&mut err, line.as_bytes());
        }
        return None;
    }
    Some(
        Typesetter::open(
            &format!("{dir}/EBGaramond-Regular.ttf"),
            &format!("{dir}/EBGaramond-Italic.ttf"),
            &format!("{dir}/NotoSerifCJKsc-Regular.otf"),
        )
        .unwrap(),
    )
}

fn input(xhtml: &str) -> ChapterInput<'_> {
    ChapterInput {
        xhtml,
        stylesheet: "",
        base_href: "chapter.xhtml",
        measure: None,
        font_size: None,
        tune: superreader_typeset::BreakTune::default(),
        bold_word_starts: false,
    }
}

#[test]
fn a_sentence_names_the_same_character_after_reflow() {
    let Some(setter) = fonts() else {
        return;
    };
    let mut words = String::new();
    for index in 0..90 {
        words.push_str(&format!("word{index} "));
    }
    let xhtml = format!("<p>{words}</p>");
    let chapter = input(&xhtml);
    let wide = setter
        .compose_box(chapter, Some((PAPERBACK_WIDTH, PAPERBACK_HEIGHT)))
        .unwrap();
    let narrow = setter.compose_box(chapter, Some((280.0, 420.0))).unwrap();
    let plain = setter.compose(chapter).unwrap();
    assert_eq!(plain.page_width(), wide.page_width());
    assert_eq!(plain.page_height(), wide.page_height());
    assert_eq!(plain.plain(), wide.plain());
    assert_eq!(superreader_typeset::chapter_plain(&xhtml), wide.plain());
    let sentence = wide.plain().find("word40").unwrap() as u32;
    assert_eq!(wide.sentence_slice(sentence, sentence + 6), "word40");
    assert_eq!(narrow.sentence_slice(sentence, sentence + 6), "word40");
    assert!(narrow.report().page_count >= wide.report().page_count);
    assert_eq!(
        wide.page_of_sentence(sentence),
        wide.page_of_sentence(sentence)
    );
    let wide_page = wide.page_of_sentence(sentence);
    let narrow_page = narrow.page_of_sentence(sentence);
    assert!(narrow_page >= wide_page);
    assert_eq!(narrow.page_width(), 280);
}
