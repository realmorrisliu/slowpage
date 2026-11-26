use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("{err}");
            ExitCode::from(1)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let src_path = args
        .next()
        .ok_or("usage: spike-icon <png|svg> <iconset> <preview>")?;
    let iconset = args
        .next()
        .ok_or("usage: spike-icon <png|svg> <iconset> <preview>")?;
    let preview = args
        .next()
        .ok_or("usage: spike-icon <png|svg> <iconset> <preview>")?;
    let iconset_path = PathBuf::from(iconset);
    let preview_path = PathBuf::from(preview);
    if src_path
        .rsplit('.')
        .next()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("png"))
    {
        let bytes = std::fs::read(&src_path).map_err(|err| format!("{src_path}: {err}"))?;
        spike_icon::write_set_png(&bytes, &iconset_path, &preview_path)
    } else {
        let svg = std::fs::read_to_string(&src_path).map_err(|err| format!("{src_path}: {err}"))?;
        spike_icon::write_set(&svg, &iconset_path, &preview_path)
    }
}
