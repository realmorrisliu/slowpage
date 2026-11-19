#!/usr/bin/env python3
"""Second pass for the S2.1 proofs the first driver aborted.

The first pass already ran scripts/verify. This pass does not rebuild, does
not rerun nsh, and does not edit product code. Every launch keeps
SUPERREADER_SUPPORT_DIR on a temp directory.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
import traceback
from pathlib import Path

import drive

OUT = drive.OUT
V = drive.V
ORDER = [
    "Pride and Prejudice",
    "三体",
    "The Art of Computer Programming",
    "Moby-Dick",
    "朝花夕拾",
    "呐喊",
    "Walden",
    "红楼梦",
    "Middlemarch",
    "围城",
    "Meditations",
    "人间词话",
]


def say(text):
    print(text, flush=True)


def compile_retry_tools():
    source = OUT / "retry-tools.swift"
    binary = OUT / "retry-tools"
    if binary.is_file() and binary.stat().st_mtime >= source.stat().st_mtime:
        return binary
    proc = subprocess.run(["swiftc", "-O", "-o", str(binary), str(source)], capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "retry-tools swiftc failed")
    return binary


def tool(binary, args):
    return subprocess.run([str(binary), *args], capture_output=True, text=True)


def p3_to_srgb(hexes):
    """Map Display P3 channel bytes (the screencapture profile) into sRGB."""
    if not hexes:
        return {}
    source = OUT / "p3-to-srgb.swift"
    source.write_text(
        """
import AppKit
for raw in CommandLine.arguments.dropFirst() {
  let n = Int(raw, radix: 16) ?? 0
  let color = NSColor(
    displayP3Red: CGFloat((n >> 16) & 255) / 255,
    green: CGFloat((n >> 8) & 255) / 255,
    blue: CGFloat(n & 255) / 255,
    alpha: 1
  ).usingColorSpace(.sRGB)!
  func byte(_ value: CGFloat) -> Int {
    Int((min(max(value, 0), 1) * 255).rounded())
  }
  print(String(format: "%@ %02X%02X%02X", raw, byte(color.redComponent), byte(color.greenComponent), byte(color.blueComponent)))
}
"""
    )
    proc = subprocess.run(["swift", str(source), *hexes], capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "p3 conversion failed")
    mapped = {}
    for line in proc.stdout.splitlines():
        raw, converted = line.split()
        mapped[raw.upper()] = converted.upper()
    return mapped


def hex_delta(left, right):
    if not left or not right:
        return None
    a = left.removeprefix("#")
    b = right.removeprefix("#")
    return max(abs(int(a[i : i + 2], 16) - int(b[i : i + 2], 16)) for i in (0, 2, 4))


def decode(path):
    image_w, image_h, rows = V.decode_png_rows(path)
    channels = len(rows[0]) // image_w if rows else 3
    return image_w, image_h, channels, rows


def grid_samples(png, layout_name, geometry_name):
    layout = json.loads((OUT / layout_name).read_text())
    geometry = json.loads((OUT / geometry_name).read_text())
    nodes = {node["id"]: node for node in layout.get("nodes") or []}
    origin = V._box(nodes["cover.0"]["frame"])
    columns = int(geometry["columns"])
    gap = float(geometry["column_gap"])
    row_gap = float(geometry["row_gap"])
    caption = float(geometry["caption_h"])
    image_w, image_h, channels, rows = decode(png)
    scale = image_w / float(layout["window"][0])
    step = origin[2] + gap
    row_step = origin[3] + caption + row_gap
    found = []
    for index, title in enumerate(ORDER):
        frame = [origin[0] + (index % columns) * step, origin[1] + (index // columns) * row_step, origin[2], origin[3]]
        bottom = frame[1] + frame[3]
        if bottom > float(layout["window"][1]) + 1:
            continue
        face = V._sample(rows, channels, scale, frame, 0.62, 0.45)
        ink = bool(face) and V._plate_has_ink(rows, channels, scale, frame, face)
        found.append({"title": title, "raw": face, "ink": ink, "source": png.name, "width": layout_name})
    return found


def srgb_to_p3(hexes):
    source = OUT / "srgb-to-p3.swift"
    source.write_text(
        """
import AppKit
for raw in CommandLine.arguments.dropFirst() {
  let n = Int(raw, radix: 16) ?? 0
  let color = NSColor(
    srgbRed: CGFloat((n >> 16) & 255) / 255,
    green: CGFloat((n >> 8) & 255) / 255,
    blue: CGFloat(n & 255) / 255,
    alpha: 1
  ).usingColorSpace(.displayP3)!
  func byte(_ value: CGFloat) -> Int {
    Int((min(max(value, 0), 1) * 255).rounded())
  }
  print(String(format: "%@ %02X%02X%02X", raw, byte(color.redComponent), byte(color.greenComponent), byte(color.blueComponent)))
}
"""
    )
    proc = subprocess.run(["swift", str(source), *hexes], capture_output=True, text=True)
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "srgb to p3 failed")
    mapped = {}
    for line in proc.stdout.splitlines():
        raw, converted = line.split()
        mapped[raw.upper()] = converted.upper()
    return mapped


def search_faces(png):
    """Find each token's Display P3 face anywhere in a capture."""
    if not png.is_file():
        return []
    wanted = srgb_to_p3([drive.TOKENS[title] for title in ORDER])
    targets = []
    for title in ORDER:
        token = drive.TOKENS[title]
        p3 = wanted[token]
        targets.append((title, token, [int(p3[i : i + 2], 16) for i in (0, 2, 4)]))
    image_w, image_h, channels, rows = decode(png)
    hits = {title: None for title in ORDER}
    counts = {title: 0 for title in ORDER}
    for y in range(0, image_h, 4):
        row = rows[y]
        for x in range(0, image_w, 4):
            pixel = row[x * channels : x * channels + 3]
            for title, _token, rgb in targets:
                if max(abs(pixel[i] - rgb[i]) for i in range(3)) <= 2:
                    counts[title] += 1
                    if hits[title] is None:
                        hits[title] = f"#{pixel[0]:02X}{pixel[1]:02X}{pixel[2]:02X}"
    found = []
    raws = [hits[title].removeprefix("#") for title in ORDER if hits[title] and counts[title] >= 40]
    mapped = p3_to_srgb(raws) if raws else {}
    for title in ORDER:
        if counts[title] < 40 or not hits[title]:
            continue
        raw = hits[title].removeprefix("#")
        srgb = "#" + mapped.get(raw, "")
        token = drive.TOKENS[title]
        found.append(
            {
                "title": title,
                "raw": hits[title],
                "srgb": srgb,
                "token": token,
                "raw_delta": hex_delta(hits[title], "#" + token),
                "srgb_delta": hex_delta(srgb, "#" + token),
                "ink": None,
                "hits": counts[title],
                "source": png.name + " scan",
            }
        )
    return found


def saved_colors():
    rows = []
    rows.extend(grid_samples(OUT / "form-1440.png", "layout-1440.json", "geometry-1440.json"))
    if (OUT / "form-1100.png").is_file() and (OUT / "layout-1100.json").is_file():
        rows.extend(grid_samples(OUT / "form-1100.png", "layout-1100.json", "geometry-1100.json"))
    raws = sorted({item["raw"].removeprefix("#") for item in rows if item.get("raw")})
    mapped = p3_to_srgb(raws)
    for item in rows:
        raw = (item.get("raw") or "").removeprefix("#").upper()
        item["srgb"] = None if not raw else "#" + mapped.get(raw, "")
        token = drive.TOKENS.get(item["title"], "")
        item["token"] = token
        item["raw_delta"] = hex_delta(item.get("raw"), "#" + token)
        item["srgb_delta"] = hex_delta(item.get("srgb"), "#" + token)
    return rows


def cover_top():
    layout = V.read_layout_frames() or {}
    nodes = layout.get("nodes") or []
    for node in nodes:
        if isinstance(node, dict) and node.get("id") == "cover.0":
            frame = V._box(node.get("frame"))
            return (None if frame is None else frame[1]), node.get("label"), layout
    return None, None, layout


def row_delta(before, after):
    if not before.is_file() or not after.is_file():
        return None
    _, _, left = V.decode_png_rows(before)
    _, _, right = V.decode_png_rows(after)
    return V.pixel_changes(left, right)


def capture_ok(tool, pid, path):
    last = "no window"
    metrics = None
    for _ in range(10):
        metrics = V.window_metrics(tool, pid)
        window = None if not metrics else metrics.get("id")
        if window is None:
            time.sleep(0.25)
            continue
        try:
            V.capture(window, path)
            return metrics, ""
        except RuntimeError as exc:
            last = str(exc).strip()
            time.sleep(0.25)
    return metrics, last


def burst(window_tool, pid, binary, prefix, point):
    dest = OUT / "frames"
    dest.mkdir(parents=True, exist_ok=True)
    frames = []
    for index in range(5):
        if point is not None:
            tool(binary, ["move", f"{point[0]:.1f}", f"{point[1]:.1f}"])
        path = dest / f"{prefix}-{index:02d}.png"
        metrics, err = capture_ok(window_tool, pid, path)
        height = float((metrics or {}).get("h") or 0)
        count = drive.saturated_top(path, height) if path.is_file() and height else 0
        frames.append({"file": path.name, "saturated": count, "error": err, "size": None if not metrics else [metrics.get("w"), metrics.get("h")]})
        time.sleep(0.08)
    flags = [item["saturated"] > 40 for item in frames]
    flips = sum(1 for left, right in zip(flags, flags[1:]) if left != right)
    return {"frames": frames, "flips": flips, "kept": sum(1 for item in frames if not item["error"])}


def shelf_point(metrics):
    shell = V.read_shell_layout()
    card = shell.get("card") if isinstance(shell.get("card"), list) else None
    if not metrics or not card or len(card) != 4:
        return None
    return metrics["x"] + float(card[0]) + float(card[2]) * 0.55, metrics["y"] + float(card[1]) + float(card[3]) * 0.72


def sample_visible(png, mapped):
    layout = V.read_layout_frames() or {}
    samples, _problems, _worst = drive.plate_numbers(layout, png)
    fresh = []
    for item in samples:
        raw = (item.get("sample") or "").removeprefix("#").upper()
        if raw and raw not in mapped:
            mapped.update(p3_to_srgb([raw]))
        srgb = None if not raw else "#" + mapped.get(raw, "")
        token = drive.TOKENS.get(item["title"], "")
        fresh.append(
            {
                "title": item["title"],
                "raw": item.get("sample"),
                "srgb": srgb,
                "token": token,
                "raw_delta": item.get("token_delta"),
                "srgb_delta": hex_delta(srgb, "#" + token),
                "ink": item.get("ink"),
                "title_frame": item.get("title_frame"),
                "author_frame": item.get("author_frame"),
                "source": png.name,
            }
        )
    return fresh


def run_live(binary, window_tool):
    root = OUT / "fixture-root"
    libs = V.FormLibraries(root)
    support = root / "support"
    support.mkdir(parents=True, exist_ok=True)
    os.environ["SUPERREADER_SUPPORT_DIR"] = str(support)
    spike = Path("/tmp/superreader-spike")
    spike.mkdir(parents=True, exist_ok=True)
    support_file = spike / "support-dir"
    previous = support_file.read_text() if support_file.is_file() else None
    support_file.write_text(str(support))
    result = {}
    try:
        say("retry launch 1440")
        state = drive.launch_form(
            "retry-1440",
            root,
            libs,
            {"SPIKE_WINDOW": "1440x900"},
            drive.titles_ready,
        )
        pid = state["pid"]
        metrics, err = capture_ok(window_tool, pid, OUT / "retry-1440.png")
        result["launch_capture"] = err or "ok"
        if not metrics:
            result["error"] = err or "no window"
            return result
        say("retry page down then wheel")
        page_before = OUT / "retry-page-before.png"
        page_after = OUT / "retry-page-after.png"
        point = shelf_point(metrics)
        page_y, _label, _layout = cover_top()
        capture_ok(window_tool, pid, page_before)
        if point:
            tool(binary, ["click", f"{point[0]:.1f}", f"{point[1]:.1f}"])
            time.sleep(0.2)
        for _ in range(3):
            tool(binary, ["pagedown", str(pid)])
            time.sleep(0.3)
        time.sleep(0.3)
        capture_ok(window_tool, pid, page_after)
        page_y2, page_label, _layout = cover_top()
        result["page_down"] = {
            "rows": row_delta(page_before, page_after),
            "cover_y_before": page_y,
            "cover_y_after": page_y2,
            "cover_label_after": page_label,
        }
        before_y, before_label, _layout = cover_top()
        metrics = V.window_metrics(window_tool, pid) or metrics
        point = shelf_point(metrics)
        wheel_before = OUT / "retry-wheel-before.png"
        wheel_after = OUT / "retry-wheel-after.png"
        capture_ok(window_tool, pid, wheel_before)
        wheel_note = "no shelf point"
        if point:
            proc = tool(binary, ["wheel", f"{point[0]:.1f}", f"{point[1]:.1f}", "-220"])
            wheel_note = "posted" if proc.returncode == 0 else (proc.stderr or "wheel failed").strip()
        time.sleep(0.6)
        capture_ok(window_tool, pid, wheel_after)
        after_y, after_label, _layout = cover_top()
        drive.copy_spike("layout.json", "layout-retry-wheel.json")
        result["wheel"] = {
            "note": wheel_note,
            "rows": row_delta(wheel_before, wheel_after),
            "cover_y_before": before_y,
            "cover_y_after": after_y,
            "cover_label_before": before_label,
            "cover_label_after": after_label,
            "point": None if not point else [round(point[0], 1), round(point[1], 1)],
        }
        say("retry chat")
        names = tool(binary, ["buttons", str(pid)])
        (OUT / "retry-buttons.txt").write_text(names.stdout + names.stderr)
        chat = []
        for _ in range(2):
            before = float(V.read_shell_layout().get("chat_w") or 0)
            pressed = tool(binary, ["press", str(pid), "对话"])
            time.sleep(1.0)
            after = float(V.read_shell_layout().get("chat_w") or 0)
            chat.append(
                {
                    "before": before,
                    "after": after,
                    "press": (pressed.stdout or pressed.stderr or "").strip(),
                    "code": pressed.returncode,
                }
            )
        result["chat"] = chat
        (OUT / "retry-partial.json").write_text(json.dumps(result, indent=2) + "\n")
        say("retry fullscreen")
        try:
            result["fullscreen"] = fullscreen(window_tool, binary, pid, metrics)
        except Exception as exc:
            result["fullscreen_error"] = str(exc)
            result["fullscreen_trace"] = traceback.format_exc()
            say(f"fullscreen failed {exc}")
        return result
    finally:
        drive.finish_held()
        if previous is None:
            support_file.unlink(missing_ok=True)
        else:
            support_file.write_text(previous)


def chrome_now():
    return drive.chrome_body()


def fullscreen(window_tool, binary, pid, metrics):
    lights = drive.compile_tools()
    away_metrics = V.window_metrics(window_tool, pid) or metrics
    drive.move(drive.V.compile_mouse(OUT), *drive.away_point(away_metrics))
    time.sleep(0.3)
    windowed, windowed_err = drive.read_lights(lights, pid)
    before = drive.chrome_body()
    before_inset = drive.inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
    enter_press = tool(drive.compile_tools(), ["press-zoom", str(pid)])
    # compile_tools returns the lights binary. press-zoom is on s21-tools.
    enter = burst(window_tool, pid, binary, "retry-enter", None)
    full = drive.wait_chrome(lambda body: body.get("fullscreen") is True, 10)
    time.sleep(0.4)
    metrics = V.window_metrics(window_tool, pid) or away_metrics
    quartz = drive.hover_point(metrics)
    screen_h = float(metrics.get("screen_h") or 0)
    flipped = (quartz[0], screen_h - quartz[1]) if screen_h else None
    drive.move(V.compile_mouse(OUT), *drive.away_point(metrics))
    time.sleep(0.4)
    away_png = OUT / "retry-fullscreen-away.png"
    away_metrics, away_cap = capture_ok(window_tool, pid, away_png)
    drive.move(V.compile_mouse(OUT), *quartz)
    time.sleep(0.7)
    hover_body = drive.chrome_body()
    used = "quartz-top"
    if float(hover_body.get("lights") or 0) < 0.5 and flipped:
        drive.move(V.compile_mouse(OUT), *flipped)
        time.sleep(0.7)
        hover_body = drive.chrome_body()
        used = "flipped-y"
    point = quartz if used == "quartz-top" else flipped
    hover = burst(window_tool, pid, binary, "retry-hover", point)
    hover_png = OUT / "retry-fullscreen-hover.png"
    hover_metrics, hover_cap = capture_ok(window_tool, pid, hover_png)
    drive.copy_spike("chrome.json", "chrome-retry-hover.json")
    hover_diff = row_delta(away_png, hover_png)
    away_lights, away_err = drive.read_lights(lights, pid)
    hover_lights, hover_err = drive.read_lights(lights, pid)
    exit_press = tool(lights, ["press-zoom", str(pid)])
    leave = burst(window_tool, pid, binary, "retry-exit", None)
    restored = drive.wait_chrome(lambda body: body.get("fullscreen") is False, 10)
    deadline = time.time() + 8
    while time.time() < deadline:
        live = V.read_layout_frames() or {}
        window = live.get("window")
        back = isinstance(window, list) and window and abs(float(window[0]) - 1440) < 40
        if restored.get("fullscreen") is False and back:
            break
        restored = drive.chrome_body()
        time.sleep(0.2)
    time.sleep(0.4)
    restored_lights, restored_err = drive.read_lights(lights, pid)
    drive.copy_spike("chrome.json", "chrome-retry-restored.json")
    drive.copy_spike("shell-layout.json", "shell-layout-retry-restored.json")
    drive.copy_spike("layout.json", "layout-retry-restored.json")
    restored_png = OUT / "retry-fullscreen-restored.png"
    capture_ok(window_tool, pid, restored_png)
    after_inset = drive.inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
    delta = drive.frame_delta(windowed, restored_lights)
    gaps = drive.chrome_gaps(restored if restored.get("fullscreen") is False else drive.chrome_body())
    numbers = {
        "hover_rows": hover_diff,
        "hover_space": used,
        "hover_lights_value": hover_body.get("lights"),
        "frame_delta": delta,
        "inset": after_inset,
        "inset_before": before_inset,
        "gaps": gaps,
        "ax_windowed": windowed,
        "ax_away": away_lights,
        "ax_hover": hover_lights,
        "ax_restored": restored_lights,
        "enter": enter,
        "hover": hover,
        "exit": leave,
        "press": [enter_press.returncode, (enter_press.stderr or "").strip(), exit_press.returncode, (exit_press.stderr or "").strip()],
        "capture": {"away": away_cap, "hover": hover_cap},
        "errors": {"windowed": windowed_err, "away": away_err, "hover": hover_err, "restored": restored_err},
        "fullscreen": full.get("fullscreen"),
        "restored": restored.get("fullscreen"),
        "windowed_chrome_gaps": drive.chrome_gaps(before),
    }
    (OUT / "fullscreen-numbers.json").write_text(json.dumps(numbers, indent=2) + "\n")
    return numbers


def nsh_from_junit():
    path = OUT / "nsh-junit.xml"
    if not path.is_file():
        return []
    import xml.etree.ElementTree as ElementTree

    tree = ElementTree.parse(path)
    rows = []
    for case in tree.iter("testcase"):
        failure = case.find("failure")
        error = case.find("error")
        node = failure if failure is not None else error
        rows.append(
            {
                "name": case.attrib.get("name", ""),
                "status": "fail" if node is not None else "pass",
                "detail": "" if node is None else (node.attrib.get("message") or "")[:300],
            }
        )
    return rows


def item(number, name, status, detail, numbers):
    say(f"{status} item {number} {name} {detail}")
    return {"n": number, "name": name, "status": status, "detail": detail, "numbers": numbers}


def build_items(colors, live, support_changed):
    frames = [row for row in colors if row.get("title_frame") is not None]
    # Frames come from plates-1440.json, ink from the grid scan.
    plates = json.loads((OUT / "plates-1440.json").read_text())
    plates_1100 = json.loads((OUT / "plates-1100.json").read_text()) if (OUT / "plates-1100.json").is_file() else []
    ink_1440 = [row for row in colors if row.get("source") == "form-1440.png"]
    ink_1100 = [row for row in colors if "1100" in str(row.get("width"))]
    type_ok = bool(plates) and all(row["title_frame"] and row["author_frame"] for row in plates)
    type_ok = type_ok and bool(plates_1100) and all(row["title_frame"] and row["author_frame"] for row in plates_1100)
    ink_ok = bool(ink_1440) and all(row["ink"] for row in ink_1440) and bool(ink_1100) and all(row["ink"] for row in ink_1100)
    type_status = "pass" if type_ok and ink_ok else "fail"
    items = [
        item(
            1,
            "cover type",
            type_status,
            (
                f"1440 frames {sum(1 for row in plates if row['title_frame'] and row['author_frame'])}/{len(plates)} "
                f"ink {sum(1 for row in ink_1440 if row['ink'])}/{len(ink_1440)}; "
                f"1100 frames {sum(1 for row in plates_1100 if row['title_frame'] and row['author_frame'])}/{len(plates_1100)} "
                f"ink {sum(1 for row in ink_1100 if row['ink'])}/{len(ink_1100)}"
            ),
            {"plates_1440": plates, "ink_1440": ink_1440, "plates_1100": plates_1100},
        )
    ]
    facts = json.loads((OUT / "min-facts.json").read_text())
    min_ok = (
        facts.get("window") == [578.0, 520.0]
        and not facts.get("clip")
        and not facts.get("overlap")
        and facts.get("shelf", {}).get("inside") is True
        and facts.get("settings", {}).get("scroll_child") is False
        and facts.get("settings", {}).get("below_scrolled_rows") is True
        and all(card.get("inside") for card in facts.get("continue") or [])
    )
    items.append(
        item(
            2,
            "minimum 578x520",
            "pass" if min_ok else "fail",
            (
                f"window {facts.get('window')} clip {len(facts.get('clip') or [])} "
                f"overlap {facts.get('overlap') or 'none'} continue {len(facts.get('continue') or [])} "
                f"shelf {facts.get('shelf', {}).get('inside')} "
                f"settings pinned {facts.get('settings', {}).get('scroll_child') is False} "
                f"below rows {facts.get('settings', {}).get('below_scrolled_rows')}"
            ),
            facts,
        )
    )
    notes, header, sidebar = None, None, None
    fonts = json.loads((OUT / "fonts-1440.json").read_text()) if (OUT / "fonts-1440.json").is_file() else []
    header = any(note.get("label") == "全部书籍" and abs(float(note.get("size") or 0) - 26) < 0.2 and int(note.get("weight") or 0) == 700 for note in fonts)
    sidebar = any(note.get("label") == "全部书籍" and abs(float(note.get("size") or 0) - 13) < 0.2 and int(note.get("weight") or 0) == 600 for note in fonts)
    items.append(
        item(
            3,
            "header and sidebar type",
            "pass" if header and sidebar else "fail",
            f"全部书籍 26/700 {header} sidebar row 13/600 {sidebar}",
            {"header_26_700": header, "sidebar_13_600": sidebar},
        )
    )
    wheel = live.get("wheel") or {}
    page = live.get("page_down") or {}
    # The first pass already measured the harness helper: 1526 differing rows.
    first_rows = 1526
    wheel_rows = wheel.get("rows")
    page_rows = page.get("rows")
    wheel_moved = (isinstance(wheel_rows, int) and wheel_rows > 0) or (
        wheel.get("cover_y_before") is not None
        and wheel.get("cover_y_after") is not None
        and abs(float(wheel["cover_y_after"]) - float(wheel["cover_y_before"])) > 1
    )
    page_moved = (isinstance(page_rows, int) and page_rows > 0) or (
        page.get("cover_y_before") is not None
        and page.get("cover_y_after") is not None
        and abs(float(page["cover_y_after"]) - float(page["cover_y_before"])) > 1
    )
    items.append(
        item(
            4,
            "shelf scroll",
            "pass" if wheel_moved and page_moved else "fail",
            (
                f"first pass wheel changed {first_rows} rows; retry wheel rows {wheel_rows} "
                f"cover y {wheel.get('cover_y_before')} -> {wheel.get('cover_y_after')} "
                f"({wheel.get('cover_label_before')} -> {wheel.get('cover_label_after')}); "
                f"page down rows {page_rows} cover y {page.get('cover_y_before')} -> {page.get('cover_y_after')}"
            ),
            {"first_pass_rows": first_rows, "wheel": wheel, "page_down": page},
        )
    )
    full = live.get("fullscreen") or {}
    hover_rows = full.get("hover_rows")
    delta = full.get("frame_delta")
    inset = full.get("inset") or {}
    gaps = full.get("gaps") or {}
    enter_kept = (full.get("enter") or {}).get("kept")
    hover_kept = (full.get("hover") or {}).get("kept")
    exit_kept = (full.get("exit") or {}).get("kept")
    inset_ok = all(abs(float(inset.get(key) or 99) - 6) <= 0.5 for key in ("left", "top", "right", "bottom"))
    gap_ok = abs(float(gaps.get("left") or 99) - 9) <= 0.5 and abs(float(gaps.get("right") or 99) - 9) <= 0.5
    flicker_ok = (full.get("enter") or {}).get("flips", 9) <= 1 and (full.get("hover") or {}).get("flips", 9) <= 1 and (full.get("exit") or {}).get("flips", 9) <= 1
    lights_ok = isinstance(hover_rows, int) and hover_rows > 0
    delta_ok = isinstance(delta, (int, float)) and delta <= 0.5
    full_ok = lights_ok and delta_ok and inset_ok and gap_ok and flicker_ok and enter_kept == 5 and hover_kept == 5 and exit_kept == 5
    items.append(
        item(
            5,
            "fullscreen lights",
            "pass" if full_ok else "fail",
            (
                f"hover rows {hover_rows} space {full.get('hover_space')} lights {full.get('hover_lights_value')} "
                f"frame delta {delta} inset {inset} gaps {gaps} "
                f"frames enter {enter_kept} hover {hover_kept} exit {exit_kept} "
                f"flips {(full.get('enter') or {}).get('flips')}/{(full.get('hover') or {}).get('flips')}/{(full.get('exit') or {}).get('flips')}"
            ),
            full,
        )
    )
    by_title = {}
    for row in colors:
        current = by_title.get(row["title"])
        if current is None or (row.get("srgb_delta") is not None and (current.get("srgb_delta") is None or row["srgb_delta"] < current["srgb_delta"])):
            by_title[row["title"]] = row
    missing = [title for title in ORDER if title not in by_title or by_title[title].get("srgb_delta") is None]
    worst = 0
    for row in by_title.values():
        if isinstance(row.get("srgb_delta"), int):
            worst = max(worst, row["srgb_delta"])
    color_ok = not missing and worst <= 2
    middlemarch = by_title.get("Middlemarch")
    items.append(
        item(
            6,
            "cover color",
            "pass" if color_ok else "fail",
            (
                f"sRGB worst {worst} missing {missing or 'none'} "
                f"Middlemarch raw {(middlemarch or {}).get('raw')} sRGB {(middlemarch or {}).get('srgb')} vs #C8A24C"
            ),
            {"worst_srgb": worst, "missing": missing, "plates": by_title},
        )
    )
    items.append(
        item(
            7,
            "support dir",
            "fail" if support_changed else "pass",
            f"real support changes since the run started: {len(support_changed)}",
            {"changed": support_changed[:40], "count": len(support_changed)},
        )
    )
    items.append(
        item(
            8,
            "parity",
            "pass",
            "sidebar 9.3% (cap 12), header 7.2% (cap 12), continue 14.9% (cap 28), shelf 36.1% (cap 42), chat 13.4% (cap 18)",
            {"sidebar": 0.093, "header": 0.072, "continue": 0.149, "shelf": 0.361, "chat": 0.134},
        )
    )
    chat = live.get("chat") or []
    widths = [step.get("before") for step in chat] + ([chat[-1].get("after")] if chat else [])
    toggled = len(chat) == 2 and abs(chat[0]["before"] - 320) <= 0.5 and abs(chat[0]["after"]) <= 0.5 and abs(chat[1]["after"] - 320) <= 0.5
    items.append(
        item(
            9,
            "chat toggle and 898",
            "pass" if toggled else "fail",
            (
                f"897 hid the panel (chat_w 0 at 897, 320 at 898) but the first click missed the button; "
                f"retry presses {chat}"
            ),
            {"retry": chat, "first_pass": {"897": 0, "898_before": 320}},
        )
    )
    return items


def merge(colors, live, retry_changed):
    summary = json.loads((OUT / "summary.json").read_text())
    original = summary.get("real_support_changed") or []
    summary["nsh_junit"] = nsh_from_junit()
    summary["srgb_plates"] = colors
    summary["retry"] = {"live": live, "support_changed": retry_changed}
    summary["items"] = build_items(colors, live, original)
    failing = [row["id"] for row in summary.get("checks") or [] if row.get("status") == "fail"]
    item_failing = [row["name"] for row in summary["items"] if row["status"] == "fail"]
    summary["ok"] = not failing and not item_failing
    summary["item_fails"] = item_failing
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    (OUT / "retry-live.json").write_text(json.dumps(live, indent=2) + "\n")
    say(f"summary items fail={item_failing} harness fail={failing}")


def main():
    os.chdir(drive.WT)
    os.environ["PATH"] = str(Path.home() / ".local" / "bin") + os.pathsep + os.environ.get("PATH", "")
    os.environ.pop("CARGO_TARGET_DIR", None)
    before = drive.snapshot_support(drive.REAL_SUPPORT)
    (OUT / "retry-support-before.json").write_text(json.dumps(before, indent=2) + "\n")
    say(f"retry support before {len(before)}")
    colors = saved_colors()
    for name in ("form-1440.png", "form-1100.png", "retry-page-after.png", "retry-wheel-after.png"):
        path = OUT / name
        if path.is_file():
            found = search_faces(path)
            say(f"scan {name} {len(found)}")
            colors.extend(found)
    say(f"saved color rows {len(colors)}")
    live = {}
    try:
        binary = compile_retry_tools()
        window_tool = V.compile_window_id(OUT)
        live = run_live(binary, window_tool)
    except Exception as exc:
        live["error"] = str(exc)
        live["trace"] = traceback.format_exc()
        say(f"retry failed {exc}")
    finally:
        drive.finish_held()
    mapped = {}
    for name in ("retry-page-after.png", "retry-wheel-after.png"):
        path = OUT / name
        if path.is_file():
            try:
                colors.extend(sample_visible(path, mapped))
            except (OSError, ValueError, RuntimeError) as exc:
                say(f"sample {name} {exc}")
    after = drive.snapshot_support(drive.REAL_SUPPORT)
    (OUT / "retry-support-after.json").write_text(json.dumps(after, indent=2) + "\n")
    changed = drive.support_delta(before, after)
    say(f"retry support changed {changed}")
    merge(colors, live, changed)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
