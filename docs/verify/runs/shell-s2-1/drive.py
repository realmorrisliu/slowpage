#!/usr/bin/env python3
"""S2.1 Mac verify driver. Scores scripts/verify and the nine real-app proofs.

Product code is not modified. Launches use a temp SUPERREADER_SUPPORT_DIR.
"""

from __future__ import annotations

import importlib.util
from importlib.machinery import SourceFileLoader
import json
import os
import shutil
import subprocess
import sys
import time
from pathlib import Path

WT = Path(__file__).resolve().parents[4]
OUT = Path(__file__).resolve().parent
REAL_SUPPORT = Path.home() / "Library/Application Support/app.slowpage.reader"
TOKENS = {
    "Pride and Prejudice": "7D8C6A",
    "Moby-Dick": "1F3A57",
    "Walden": "2F4A3C",
    "Middlemarch": "C8A24C",
    "Meditations": "4A3C5A",
    "三体": "16212B",
    "The Art of Computer Programming": "EEE6D3",
    "朝花夕拾": "B4553C",
    "呐喊": "1C1C1C",
    "红楼梦": "8A2B2B",
    "围城": "DDE3E7",
    "人间词话": "F2EEE4",
}


def load_verify():
    path = WT / "scripts" / "verify"
    loader = SourceFileLoader("srverify", str(path))
    spec = importlib.util.spec_from_loader("srverify", loader)
    module = importlib.util.module_from_spec(spec)
    loader.exec_module(module)
    return module


V = load_verify()
HELD = {"state": None}


def say(text):
    print(text, flush=True)


def snapshot_support(root: Path):
    rows = []
    if not root.exists():
        return rows
    paths = [root, *sorted(p for p in root.rglob("*"))]
    for path in paths:
        st = path.lstat()
        rows.append(
            {
                "path": str(path.relative_to(root)) if path != root else ".",
                "mtime_ns": st.st_mtime_ns,
                "size": st.st_size,
                "mode": st.st_mode,
            }
        )
    return rows


def support_delta(before, after):
    bmap = {row["path"]: row for row in before}
    amap = {row["path"]: row for row in after}
    changed = []
    for key in sorted(set(bmap) | set(amap)):
        left = bmap.get(key)
        right = amap.get(key)
        if left is None or right is None or left["mtime_ns"] != right["mtime_ns"] or left["size"] != right["size"]:
            changed.append(key)
    return changed


def copy_spike(name, dest):
    src = Path("/tmp/superreader-spike") / name
    if not src.is_file():
        return False
    target = OUT / dest
    target.parent.mkdir(parents=True, exist_ok=True)
    shutil.copy(src, target)
    return True


def read_json(path):
    try:
        return json.loads(Path(path).read_text())
    except (OSError, json.JSONDecodeError):
        return None


def chrome_body():
    return read_json("/tmp/superreader-spike/chrome.json") or {}


def compile_tools():
    source = OUT / "s21-tools.swift"
    binary = OUT / "s21-tools"
    source.write_text(
        r"""
import AppKit
import ApplicationServices
import CoreGraphics

let args = CommandLine.arguments
guard args.count > 1 else { exit(2) }
let cmd = args[1]

func postKey(_ code: CGKeyCode) {
  let src = CGEventSource(stateID: .hidSystemState)
  let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true)
  let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false)
  down?.post(tap: .cghidEventTap)
  up?.post(tap: .cghidEventTap)
}

func postClick(_ x: Double, _ y: Double) {
  let p = CGPoint(x: x, y: y)
  let src = CGEventSource(stateID: .hidSystemState)
  let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: p, mouseButton: .left)
  let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: p, mouseButton: .left)
  down?.post(tap: .cghidEventTap)
  up?.post(tap: .cghidEventTap)
}

if cmd == "click" {
  guard args.count > 3, let x = Double(args[2]), let y = Double(args[3]) else { exit(2) }
  postClick(x, y)
  exit(0)
}
if cmd == "pagedown" {
  postKey(121)
  exit(0)
}

guard args.count > 2, let pid = pid_t(args[2]) else { exit(2) }
let app = AXUIElementCreateApplication(pid)
var raw: CFTypeRef?
guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &raw) == .success,
      let windows = raw as? [AXUIElement], !windows.isEmpty else {
  fputs("no window\n", stderr)
  exit(1)
}

func axValue(_ el: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(el, name as CFString, &value) == .success else { return nil }
  return value
}
func box(_ el: AXUIElement) -> (CGFloat, CGFloat, CGFloat, CGFloat)? {
  guard let p = axValue(el, kAXPositionAttribute as String),
        let s = axValue(el, kAXSizeAttribute as String),
        CFGetTypeID(p) == AXValueGetTypeID(),
        CFGetTypeID(s) == AXValueGetTypeID() else { return nil }
  var pos = CGPoint.zero
  var size = CGSize.zero
  guard AXValueGetValue(p as! AXValue, .cgPoint, &pos),
        AXValueGetValue(s as! AXValue, .cgSize, &size) else { return nil }
  return (pos.x, pos.y, size.width, size.height)
}

var found: [String: AXUIElement] = [:]
var frames: [String: (CGFloat, CGFloat, CGFloat, CGFloat)] = [:]
var origin: (CGFloat, CGFloat, CGFloat, CGFloat)?
var best: CGFloat = -1
func walk(_ el: AXUIElement, _ depth: Int) {
  if depth > 16 { return }
  let role = axValue(el, kAXSubroleAttribute as String) as? String ?? ""
  if role == "AXCloseButton" || role == "AXMinimizeButton" || role == "AXZoomButton" || role == "AXFullScreenButton" {
    found[role] = el
    if let frame = box(el) { frames[role] = frame }
  }
  var kids: CFTypeRef?
  if AXUIElementCopyAttributeValue(el, kAXChildrenAttribute as CFString, &kids) == .success,
     let kids = kids as? [AXUIElement] {
    for kid in kids { walk(kid, depth + 1) }
  }
}
for window in windows {
  walk(window, 0)
  if let frame = box(window) {
    let area = frame.2 * frame.3
    if area > best { best = area; origin = frame }
  }
}

if cmd == "press-zoom" {
  let zoom = found["AXZoomButton"] ?? found["AXFullScreenButton"]
  guard let zoom else { fputs("no zoom\n", stderr); exit(1) }
  let err = AXUIElementPerformAction(zoom, kAXPressAction as CFString)
  if err != .success { fputs("press \(err.rawValue)\n", stderr); exit(1) }
  exit(0)
}

guard cmd == "lights", let origin else { fputs("no origin\n", stderr); exit(1) }
func local(_ item: (CGFloat, CGFloat, CGFloat, CGFloat)) -> String {
  let minX = item.0 - origin.0
  let minY = item.1 - origin.1
  return "\(minX) \(minX + item.2) \(minY) \(minY + item.3)"
}
let close = frames["AXCloseButton"]
let mini = frames["AXMinimizeButton"]
let zoom = frames["AXZoomButton"] ?? frames["AXFullScreenButton"]
if close == nil || mini == nil || zoom == nil {
  fputs("missing \(Array(frames.keys))\n", stderr)
  exit(1)
}
print("close \(local(close!)) mini \(local(mini!)) zoom \(local(zoom!))")
"""
    )
    if binary.is_file() and binary.stat().st_mtime >= source.stat().st_mtime:
        return binary
    proc = subprocess.run(
        ["swiftc", "-O", "-o", str(binary), str(source)],
        capture_output=True,
        text=True,
    )
    if proc.returncode != 0:
        raise RuntimeError(proc.stderr or "s21-tools swiftc failed")
    return binary


def tool_run(binary, args):
    return subprocess.run([str(binary), *args], capture_output=True, text=True)


def parse_lights(text):
    parts = text.split()
    if len(parts) != 15 or parts[0] != "close":
        return None
    keys = (
        "close_min_x", "close_max_x", "close_min_y", "close_max_y",
        "mini_min_x", "mini_max_x", "mini_min_y", "mini_max_y",
        "zoom_min_x", "zoom_max_x", "zoom_min_y", "zoom_max_y",
    )
    try:
        values = [float(parts[i]) for i in (1, 2, 3, 4, 6, 7, 8, 9, 11, 12, 13, 14)]
    except ValueError:
        return None
    return dict(zip(keys, values))


def read_lights(binary, pid):
    proc = tool_run(binary, ["lights", str(pid)])
    if proc.returncode != 0:
        return None, (proc.stderr or proc.stdout or "lights failed").strip()
    frame = parse_lights(proc.stdout)
    if frame is None:
        return None, proc.stdout.strip() or "bad lights line"
    return frame, ""


def finish_held():
    state = HELD["state"]
    HELD["state"] = None
    if state is not None:
        V.finish_launch(state)


def launch_form(name, root, libs, extra, ready, timeout=45):
    finish_held()
    V.clear_layout_files()
    state = V.launch(
        OUT,
        name,
        V._form_env(root, libs, extra),
        ready,
        timeout,
    )
    HELD["state"] = state
    return state


def titles_ready(names):
    return "Pride and Prejudice" in names and "人间词话" in names


def wait_chrome(pred, timeout=8):
    deadline = time.time() + timeout
    last = {}
    while time.time() < deadline:
        body = chrome_body()
        if isinstance(body, dict):
            last = body
            if pred(body):
                return body
        time.sleep(0.12)
    return last


def plate_numbers(layout, png):
    problems = V.plate_report(layout, png)
    nodes = layout.get("nodes") if isinstance(layout, dict) else None
    by_id = {}
    if isinstance(nodes, list):
        by_id = {node.get("id"): node for node in nodes if isinstance(node, dict)}
    rows = None
    channels = 3
    scale = 1.0
    if png.is_file():
        image_w, _image_h, rows = V.decode_png_rows(png)
        channels = len(rows[0]) // image_w if rows else 3
        window = layout.get("window") if isinstance(layout, dict) else None
        if isinstance(window, list) and window and float(window[0]) > 0:
            scale = image_w / float(window[0])
    samples = []
    worst = 0
    for node in by_id.values():
        if not node.get("plate"):
            continue
        name = str(node.get("id") or "cover")
        label = str(node.get("label") or "")
        frame = V._box(node.get("frame"))
        title = by_id.get(f"{name}.type.title")
        author = by_id.get(f"{name}.type.author")
        face = None
        if rows is not None and frame is not None:
            face = V._sample(rows, channels, scale, frame, 0.22, 0.16)
        want = str(node.get("plate_rgb") or TOKENS.get(label) or "")
        token = TOKENS.get(label, "")
        delta = V._hex_delta(face, "#" + want) if face and want else None
        token_delta = V._hex_delta(face, "#" + token) if face and token else None
        if isinstance(token_delta, int):
            worst = max(worst, token_delta)
        ink = bool(frame and rows is not None and V._plate_has_ink(rows, channels, scale, frame, face))
        title_in = False
        author_in = False
        if frame is not None and isinstance(title, dict):
            title_in = V._contains(frame, V._box(title.get("frame")) or [0, 0, 0, 0])
        if frame is not None and isinstance(author, dict):
            author_in = V._contains(frame, V._box(author.get("frame")) or [0, 0, 0, 0])
        samples.append(
            {
                "id": name,
                "title": label,
                "token": token,
                "plate_rgb": want,
                "sample": face,
                "token_delta": token_delta,
                "plate_delta": delta,
                "title_frame": title_in,
                "author_frame": author_in,
                "ink": ink,
            }
        )
    samples.sort(key=lambda item: item["id"])
    return samples, problems, worst


def font_pair():
    path = Path("/tmp/superreader-spike/fonts.json")
    notes = read_json(path) or []
    rows = [
        {"size": note.get("size"), "weight": note.get("weight")}
        for note in notes
        if isinstance(note, dict) and note.get("label") == "全部书籍"
    ]
    header = any(abs(float(row["size"] or 0) - 26) <= 0.1 and int(row["weight"] or 0) == 700 for row in rows)
    sidebar = any(abs(float(row["size"] or 0) - 13) <= 0.1 and int(row["weight"] or 0) == 600 for row in rows)
    return rows, header, sidebar


def header_ink(png):
    if not png.is_file():
        return 0
    width, height, rows = V.decode_png_rows(png)
    channels = len(rows[0]) // width
    ink = 0
    y0, y1 = int(height * 0.04), int(height * 0.16)
    x0, x1 = int(width * 0.18), int(width * 0.55)
    for y in range(y0, y1, 2):
        row = rows[y]
        for x in range(x0, x1, 2):
            i = x * channels
            if row[i] < 80 and row[i + 1] < 80 and row[i + 2] < 80:
                ink += 1
    return ink


def saturated_top(path, window_h):
    width, height, rows = V.decode_png_rows(path)
    channels = len(rows[0]) // width
    scale = height / window_h if window_h else 2.0
    band = max(1, int(round(56 * scale)))
    count = 0
    for y in range(min(band, height)):
        row = rows[y]
        for x in range(width):
            i = x * channels
            red, green, blue = row[i], row[i + 1], row[i + 2]
            red_disc = red > 180 and green < 140 and blue < 140
            yellow_disc = red > 180 and green > 150 and blue < 120
            green_disc = green > 150 and red < 150 and blue < 140
            if red_disc or yellow_disc or green_disc:
                count += 1
    return count


def burst(tool, pid, dest, prefix, mouse, point, n=5):
    dest.mkdir(parents=True, exist_ok=True)
    names = []
    counts = []
    sizes = []
    for index in range(n):
        if point is not None:
            subprocess.run([str(mouse), f"{point[0]:.1f}", f"{point[1]:.1f}"], capture_output=True)
        metrics = V.window_metrics(tool, pid) or {}
        window = metrics.get("id")
        path = dest / f"{prefix}-{index:02d}.png"
        if window is None:
            names.append(None)
            counts.append(0)
            sizes.append(None)
            continue
        V.capture(window, path)
        window_h = float(metrics.get("h") or 0)
        names.append(path.name)
        counts.append(saturated_top(path, window_h))
        sizes.append([metrics.get("w"), metrics.get("h")])
        time.sleep(0.08)
    flags = [count > 40 for count in counts]
    flips = sum(1 for left, right in zip(flags, flags[1:]) if left != right)
    return {"frames": names, "saturated": counts, "sizes": sizes, "flips": flips}


def frame_delta(left, right):
    if not left or not right:
        return None
    deltas = [abs(left[key] - right[key]) for key in left if key in right]
    if not deltas:
        return None
    return max(deltas)


def light_gaps(frame):
    if not frame:
        return None
    between_close_mini = frame["mini_min_x"] - frame["close_max_x"]
    between_mini_zoom = frame["zoom_min_x"] - frame["mini_max_x"]
    return {
        "close_min_x": frame["close_min_x"],
        "button_gap_close_mini": between_close_mini,
        "button_gap_mini_zoom": between_mini_zoom,
    }


def chrome_gaps(body):
    try:
        left = float(body.get("close_min_x"))
        zoom_max = float(body.get("zoom_max_x"))
        bar = float(body.get("bar_w") or body.get("light_w") or 0)
    except (TypeError, ValueError):
        return None
    right = bar - zoom_max if bar else None
    return {"left": left, "right": right, "bar_w": bar, "light_w": body.get("light_w")}


def inset_of(shell, layout):
    card = shell.get("card") if isinstance(shell, dict) else None
    window = layout.get("window") if isinstance(layout, dict) else None
    if not isinstance(card, list) or len(card) != 4 or not isinstance(window, list) or len(window) < 2:
        return None
    left, top, width, height = [float(item) for item in card]
    frame_w, frame_h = float(window[0]), float(window[1])
    sidebar = float(shell.get("sidebar_w") or 0)
    chat = float(shell.get("chat_w") or 0)
    return {
        "left": left - sidebar,
        "top": top,
        "right": (frame_w - chat) - (left + width),
        "bottom": frame_h - (top + height),
        "sidebar_w": sidebar,
        "chat_w": chat,
        "window": [frame_w, frame_h],
    }


def move(mouse, x, y):
    subprocess.run([str(mouse), f"{x:.1f}", f"{y:.1f}"], capture_output=True)


def hover_point(metrics):
    return metrics["x"] + 36.0, metrics["y"] + 8.0


def away_point(metrics):
    return metrics["x"] + metrics["w"] * 0.45, metrics["y"] + metrics["h"] * 0.72


def row(check_id, status, detail, evidence=None, numbers=None):
    item = V.check(check_id, "shell", "window", status, detail, evidence)
    if numbers is not None:
        item["numbers"] = numbers
    say(f"{status} {check_id} {detail}")
    return item


def min_facts(layout):
    nodes = layout.get("nodes") if isinstance(layout, dict) else []
    by_label = {}
    continue_cards = []
    shelf = None
    settings = None
    scrolling = []
    if isinstance(nodes, list):
        for node in nodes:
            if not isinstance(node, dict):
                continue
            label = str(node.get("label") or "")
            if label:
                by_label.setdefault(label, node)
            if str(node.get("id") or "").startswith("continue."):
                continue_cards.append(node)
            if node.get("id") == "shelf.row":
                shelf = node
            if node.get("id") == "sidebar.settings":
                settings = node
            if node.get("scroll_child") and str(node.get("id") or "").startswith("sidebar.row"):
                scrolling.append(node.get("label"))
    window = layout.get("window") if isinstance(layout, dict) else None
    window_h = float(window[1]) if isinstance(window, list) and len(window) > 1 else 0
    problems = V.clipping_report(layout)
    overlap = [item for item in problems if "overlaps" in item]
    settings_vs = [item for item in overlap if "设置" in item or "sidebar.settings" in item]
    cards_inside = []
    for card in continue_cards:
        frame = V._box(card.get("frame"))
        inside = bool(frame and frame[1] >= -0.5 and frame[1] + frame[3] <= window_h + 0.5)
        cards_inside.append({"id": card.get("id"), "label": card.get("label"), "frame": frame, "inside": inside})
    shelf_frame = V._box(shelf.get("frame")) if isinstance(shelf, dict) else None
    shelf_inside = bool(shelf_frame and shelf_frame[1] + shelf_frame[3] <= window_h + 0.5)
    settings_frame = V._box(settings.get("frame")) if isinstance(settings, dict) else None
    scroll_bottoms = []
    for node in nodes if isinstance(nodes, list) else []:
        if isinstance(node, dict) and node.get("scroll_child") and str(node.get("id") or "").startswith("sidebar.row."):
            frame = V._box(node.get("frame"))
            if frame:
                scroll_bottoms.append(frame[1] + frame[3])
    settings_below = None
    if settings_frame and scroll_bottoms:
        settings_below = settings_frame[1] + 0.5 >= max(scroll_bottoms)
    return {
        "window": window,
        "clip": problems,
        "overlap": overlap,
        "settings_overlap": settings_vs,
        "continue": cards_inside,
        "shelf": {"frame": shelf_frame, "inside": shelf_inside},
        "settings": {
            "frame": settings_frame,
            "scroll_child": None if not isinstance(settings, dict) else settings.get("scroll_child"),
            "below_scrolled_rows": settings_below,
        },
        "scrolling_rows": scrolling,
        "reading_frame": None if "正在读" not in by_label else by_label["正在读"].get("frame"),
        "settings_label_frame": None if "设置" not in by_label else by_label["设置"].get("frame"),
    }


def capture_pid(tool, pid, png):
    metrics = V.window_metrics(tool, pid)
    if not metrics or "id" not in metrics:
        return None
    V.capture(metrics["id"], png)
    return metrics


def main():
    os.chdir(WT)
    os.environ["PATH"] = str(Path.home() / ".local" / "bin") + os.pathsep + os.environ.get("PATH", "")
    os.environ.pop("CARGO_TARGET_DIR", None)
    before = snapshot_support(REAL_SUPPORT)
    (OUT / "support-before.json").write_text(json.dumps(before, indent=2) + "\n")
    say(f"support before {len(before)} paths")
    rows = []
    try:
        V.self_test()
        rows.append(V.check("verify-self-test", "shell", "doctor", "pass", "scripts/verify self-test passed"))
    except Exception as exc:
        rows.append(V.check("verify-self-test", "shell", "doctor", "fail", str(exc)))
        return finish(rows, before, 1)
    rows.extend(V.doctor())
    if any(row["status"] == "fail" for row in rows):
        return finish(rows, before, 1)
    say("building spike-shell")
    build = subprocess.run(
        [V.cargo_bin(), "build", "--release", "-p", "spike-shell"],
        cwd=V.SPIKES,
        capture_output=True,
        text=True,
    )
    (OUT / "build-shell.log").write_text(build.stdout + "\n" + build.stderr)
    if build.returncode != 0:
        tail = "\n".join((build.stderr or build.stdout).splitlines()[-20:])
        rows.append(V.check("spike-shell-build", "shell", "doctor", "fail", tail or "cargo build failed"))
        return finish(rows, before, 1)
    rows.append(V.check("spike-shell-build", "shell", "doctor", "pass", "release spike-shell"))
    say("nsh shell rows")
    try:
        rows.extend(V.run_nsh(OUT))
    except Exception as exc:
        rows.append(V.check("shell-sidebar", "shell", "nsh", "fail", str(exc)))
    say("form fixture harness")
    tool = V.compile_window_id(OUT)
    try:
        rows.extend(V.run_form_fixture(OUT, tool))
    except Exception as exc:
        rows.append(V.check("shell-form-fixture", "shell", "window", "fail", str(exc)))
        rows.append(V.check("shell-form-parity", "shell", "window", "fail", str(exc)))
    try:
        rows.extend(proofs(tool))
    except Exception as exc:
        rows.append(V.check("s21-proofs", "shell", "window", "fail", str(exc)))
        say(f"proofs failed {exc}")
    say("smoke")
    try:
        rows.extend(V.run_smoke(OUT, tool))
    except Exception as exc:
        rows.append(V.check("smoke-launch", "smoke", "window", "fail", str(exc)))
    if not any(row["id"] == "smoke-shell-click" for row in rows):
        rows.append(V.check("smoke-shell-click", "smoke", "window", "pending", V.SHELL_CLICK_GAP))
    return finish(rows, before, None)


def proofs(tool):
    items = []
    try:
        binary = compile_tools()
        mouse = V.compile_mouse(OUT)
    except Exception as exc:
        return [row("s21-tools", "fail", str(exc))]
    root = OUT / "fixture-root"
    if root.exists():
        shutil.rmtree(root)
    root.mkdir(parents=True)
    subprocess.run(
        [str(V.release_bin("library-fixture")), str(root), "form"],
        check=True,
        cwd=V.SPIKES,
    )
    libs = V.FormLibraries(root)
    support = OUT / "form-support"
    if support.exists():
        shutil.rmtree(support)
    support.mkdir()
    os.environ["SUPERREADER_SUPPORT_DIR"] = str(support)
    try:
        items.extend(prove_minimum(tool, root, libs))
        items.extend(prove_wide(tool, mouse, binary, root, libs, "1100x700", "1100"))
        items.extend(prove_breakpoints(tool, root, libs, binary))
        items.extend(prove_main(tool, mouse, binary, root, libs, support))
    finally:
        finish_held()
    glyphs = sorted(path.name for path in (support / "shell-glyphs").glob("*.png")) if (support / "shell-glyphs").is_dir() else []
    (OUT / "glyph-names.json").write_text(json.dumps(glyphs, indent=2) + "\n")
    items.append(
        row(
            "support-glyph-dir",
            "pass" if glyphs else "fail",
            f"{len(glyphs)} glyph pngs in {support / 'shell-glyphs'}",
            "glyph-names.json",
            {"count": len(glyphs), "dir": str(support / "shell-glyphs"), "names": glyphs},
        )
    )
    return items


def prove_minimum(tool, root, libs):
    height = int(V.library_min_height())
    say(f"minimum 578x{height}")
    state = launch_form(
        "proof-min",
        root,
        libs,
        {"SPIKE_WINDOW": f"578x{height}"},
        lambda names: "书库" in names
        and (geo := V.read_geometry())
        and abs(float(geo.get("content_w") or 0) - 266.0) <= 1.0,
    )
    png = OUT / "form-minimum.png"
    capture_pid(tool, state["pid"], png)
    time.sleep(0.3)
    copy_spike("layout.json", "layout-min.json")
    copy_spike("geometry.json", "geometry-min.json")
    copy_spike("shell-layout.json", "shell-layout-min.json")
    layout = V.read_layout_frames() or {}
    facts = min_facts(layout)
    (OUT / "min-facts.json").write_text(json.dumps(facts, indent=2) + "\n")
    cards_ok = bool(facts["continue"]) and all(card["inside"] for card in facts["continue"])
    ok = (
        not facts["clip"]
        and cards_ok
        and facts["shelf"]["inside"]
        and facts["settings"]["scroll_child"] is False
        and facts["settings"]["below_scrolled_rows"] is True
        and "正在读" in facts["scrolling_rows"]
    )
    detail = (
        f"window {facts['window']} clip {len(facts['clip'])} overlap {facts['overlap'] or 'none'} "
        f"continue {len(facts['continue'])} inside {cards_ok} shelf {facts['shelf']['inside']} "
        f"settings pinned {facts['settings']['scroll_child'] is False} "
        f"below rows {facts['settings']['below_scrolled_rows']}"
    )
    return [row("minimum-578x520", "pass" if ok else "fail", detail, "form-minimum.png", facts)]


def prove_wide(tool, mouse, binary, root, libs, size, tag):
    say(size)
    state = launch_form(f"proof-{tag}", root, libs, {"SPIKE_WINDOW": size}, titles_ready)
    png = OUT / f"form-{tag}.png"
    capture_pid(tool, state["pid"], png)
    time.sleep(0.4)
    copy_spike("layout.json", f"layout-{tag}.json")
    copy_spike("geometry.json", f"geometry-{tag}.json")
    copy_spike("shell-layout.json", f"shell-layout-{tag}.json")
    layout = V.read_layout_frames() or {}
    samples, problems, worst = plate_numbers(layout, png)
    (OUT / f"plates-{tag}.json").write_text(json.dumps(samples, indent=2) + "\n")
    type_ok = bool(samples) and all(item["title_frame"] and item["author_frame"] and item["ink"] for item in samples)
    color_ok = bool(samples) and all(
        isinstance(item["token_delta"], int) and item["token_delta"] <= 2 for item in samples
    )
    clip = V.clipping_report(layout)
    shell = V.read_shell_layout()
    chat = float(shell.get("chat_w") or 0)
    geo_ok, geo_detail = V.geometry_matches(V.read_geometry())
    detail = (
        f"{size} plates {len(samples)} worst token delta {worst} type {type_ok} "
        f"color {color_ok} clip {clip[:4] or 'none'} chat {chat} {geo_detail}"
    )
    status = "pass" if type_ok and color_ok and not clip and not problems else "fail"
    return [
        row(
            f"plates-{tag}",
            status,
            detail,
            png.name,
            {
                "samples": samples,
                "problems": problems,
                "worst_token_delta": worst,
                "clip": clip,
                "chat_w": chat,
                "geometry": geo_detail,
                "geometry_ok": geo_ok,
            },
        )
    ]


def prove_breakpoints(tool, root, libs, binary):
    items = []
    for width, expect in ((897, 0.0), (898, 320.0)):
        say(f"breakpoint {width}")
        state = launch_form(
            f"proof-{width}",
            root,
            libs,
            {"SPIKE_WINDOW": f"{width}x640"},
            lambda names, expect=expect: "书库" in names
            and abs(float(V.read_shell_layout().get("chat_w") or 0) - expect) <= 0.5,
        )
        png = OUT / f"form-{width}.png"
        capture_pid(tool, state["pid"], png)
        time.sleep(0.3)
        before = float(V.read_shell_layout().get("chat_w") or 0)
        clicked = ax_click(state["pid"], "对话")
        time.sleep(1.0)
        deadline = time.time() + 3
        after = before
        while time.time() < deadline:
            after = float(V.read_shell_layout().get("chat_w") or 0)
            time.sleep(0.2)
        window = (V.read_layout_frames() or {}).get("window")
        stayed = abs(after - expect) <= 0.5 and abs(before - expect) <= 0.5
        # 898 starts shown. A toggle there must hide then we only click once,
        # so 898's after is 0 when the toggle works. 897 must stay hidden.
        if width == 898:
            toggle_ok = abs(before - 320) <= 0.5 and abs(after) <= 0.5 and clicked == "clicked"
            detail = f"898 before {before} after one toggle {after} click {clicked} window {window}"
            items.append(
                row(
                    "chat-breakpoint-898",
                    "pass" if toggle_ok else "fail",
                    detail,
                    png.name,
                    {"before": before, "after": after, "clicked": clicked, "window": window},
                )
            )
        else:
            detail = f"897 before {before} after toggle {after} click {clicked} window {window}"
            items.append(
                row(
                    "chat-breakpoint-897",
                    "pass" if stayed and clicked == "clicked" else "fail",
                    detail,
                    png.name,
                    {"before": before, "after": after, "clicked": clicked, "window": window, "stayed": stayed},
                )
            )
    return items


def ax_click(pid, label):
    script = f'''
tell application "System Events"
  tell (first process whose unix id is {pid})
    set frontmost to true
    repeat with e in entire contents of window 1
      try
        set n to description of e as text
        if n is "{label}" then
          click e
          return "clicked"
        end if
      end try
      try
        set n to name of e as text
        if n is "{label}" then
          click e
          return "clicked"
        end if
      end try
    end repeat
    return "missing"
  end tell
end tell
'''
    proc = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
    if proc.returncode != 0:
        return (proc.stderr or proc.stdout or "click failed").strip()
    return proc.stdout.strip() or "missing"


def prove_main(tool, mouse, binary, root, libs, support):
    say("1440 proofs")
    items = []
    state = launch_form("proof-1440", root, libs, {"SPIKE_WINDOW": "1440x900"}, titles_ready)
    png = OUT / "form-1440.png"
    metrics = capture_pid(tool, state["pid"], png)
    time.sleep(0.5)
    copy_spike("layout.json", "layout-1440.json")
    copy_spike("fonts.json", "fonts-1440.json")
    copy_spike("geometry.json", "geometry-1440.json")
    copy_spike("shell-layout.json", "shell-layout-1440.json")
    copy_spike("chrome.json", "chrome-windowed.json")
    layout = V.read_layout_frames() or {}
    samples, problems, worst = plate_numbers(layout, png)
    (OUT / "plates-1440.json").write_text(json.dumps(samples, indent=2) + "\n")
    type_ok = bool(samples) and all(item["title_frame"] and item["author_frame"] and item["ink"] for item in samples)
    color_ok = bool(samples) and all(isinstance(item["token_delta"], int) and item["token_delta"] <= 2 for item in samples)
    middlemarch = next((item for item in samples if item["title"] == "Middlemarch"), None)
    items.append(
        row(
            "cover-type-1440",
            "pass" if type_ok else "fail",
            f"{len(samples)} plates type+ink {type_ok} problems {problems[:4] or 'none'}",
            "form-1440.png",
            {"samples": [{k: item[k] for k in ("id", "title", "title_frame", "author_frame", "ink")} for item in samples]},
        )
    )
    items.append(
        row(
            "cover-color-1440",
            "pass" if color_ok else "fail",
            f"worst token delta {worst} Middlemarch {None if not middlemarch else middlemarch['sample']} vs #C8A24C",
            "plates-1440.json",
            {"worst": worst, "middlemarch": middlemarch, "samples": samples},
        )
    )
    notes, header, sidebar = font_pair()
    ink = header_ink(png)
    font_ok, font_detail = V.read_font_notes()
    items.append(
        row(
            "header-sidebar-fonts",
            "pass" if header and sidebar and ink > 20 and font_ok else "fail",
            f"全部书籍 notes {notes} header 26/700 {header} sidebar 13/600 {sidebar} header ink {ink} {font_detail}",
            "fonts-1440.json",
            {"notes": notes, "header_26_700": header, "sidebar_13_600": sidebar, "header_ink": ink},
        )
    )
    wheel_ok, wheel_detail = V.shelf_wheel(OUT, tool, state)
    before = OUT / "page-down-before.png"
    after = OUT / "page-down-after.png"
    page_diff = 0
    page_err = ""
    metrics = V.window_metrics(tool, state["pid"]) or metrics
    if metrics and metrics.get("id"):
        V.capture(metrics["id"], before)
        point = away_point(metrics)
        tool_run(binary, ["click", str(point[0]), str(point[1])])
        time.sleep(0.3)
        for _ in range(3):
            tool_run(binary, ["pagedown"])
            time.sleep(0.35)
        fresh = V.window_metrics(tool, state["pid"]) or metrics
        if fresh.get("id"):
            V.capture(fresh["id"], after)
        try:
            _w, _h, left = V.decode_png_rows(before)
            _w, _h, right = V.decode_png_rows(after)
            page_diff = V.pixel_changes(left, right)
        except (OSError, ValueError) as exc:
            page_diff = 0
            page_err = str(exc)
    items.append(
        row(
            "shelf-wheel-and-page-down",
            "pass" if wheel_ok and page_diff > 0 else "fail",
            f"{wheel_detail}; page down changed {page_diff} pixels {page_err}",
            "shelf-wheel-after.png" if (OUT / "shelf-wheel-after.png").exists() else "page-down-after.png",
            {"page_down_pixels": page_diff, "wheel": wheel_detail},
        )
    )
    # Chat toggle at 1440, which is above the 898 breakpoint.
    chat_before = float(V.read_shell_layout().get("chat_w") or 0)
    first = ax_click(state["pid"], "对话")
    time.sleep(1.0)
    chat_hidden = float(V.read_shell_layout().get("chat_w") or 0)
    second = ax_click(state["pid"], "对话")
    time.sleep(1.0)
    chat_after = float(V.read_shell_layout().get("chat_w") or 0)
    toggle_ok = abs(chat_before - 320) <= 0.5 and abs(chat_hidden) <= 0.5 and abs(chat_after - 320) <= 0.5
    items.append(
        row(
            "chat-toggle-1440",
            "pass" if toggle_ok else "fail",
            f"chat {chat_before} then {chat_hidden} then {chat_after} clicks {first}/{second}",
            None,
            {"before": chat_before, "hidden": chat_hidden, "after": chat_after, "clicks": [first, second]},
        )
    )
    items.extend(prove_fullscreen(tool, mouse, binary, state))
    glyph_count = len(list((support / "shell-glyphs").glob("*.png"))) if (support / "shell-glyphs").is_dir() else 0
    items.append(
        row(
            "glyphs-during-1440",
            "pass" if glyph_count else "fail",
            f"{glyph_count} rasters written under the temp support dir during the 1440 launch",
            None,
            {"count": glyph_count},
        )
    )
    return items


def prove_fullscreen(tool, mouse, binary, state):
    say("fullscreen hover")
    pid = state["pid"]
    metrics = V.window_metrics(tool, pid)
    if not metrics:
        return [row("fullscreen-lights", "fail", "no window before fullscreen")]
    move(mouse, *away_point(metrics))
    time.sleep(0.3)
    windowed, windowed_err = read_lights(binary, pid)
    copy_spike("chrome.json", "chrome-before-fullscreen.json")
    before_chrome = chrome_body()
    before_inset = inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
    frames = OUT / "frames"
    enter = burst(tool, pid, frames, "enter-pre", mouse, None, n=1)
    pressed = tool_run(binary, ["press-zoom", str(pid)])
    enter = burst(tool, pid, frames, "enter", mouse, None, n=5)
    full = wait_chrome(lambda body: body.get("fullscreen") is True, 10)
    time.sleep(0.4)
    metrics = V.window_metrics(tool, pid) or metrics
    move(mouse, *away_point(metrics))
    time.sleep(0.45)
    away = OUT / "fullscreen-away.png"
    if metrics.get("id"):
        V.capture(metrics["id"], away)
    away_lights, away_err = read_lights(binary, pid)
    move(mouse, *hover_point(metrics))
    time.sleep(0.45)
    hover = burst(tool, pid, frames, "hover", mouse, hover_point(metrics), n=5)
    hover_png = OUT / "fullscreen-hover.png"
    if metrics.get("id"):
        V.capture(metrics["id"], hover_png)
    hover_lights, hover_err = read_lights(binary, pid)
    copy_spike("chrome.json", "chrome-fullscreen-hover.json")
    hover_diff = 0
    if away.exists() and hover_png.exists():
        try:
            _w, _h, left = V.decode_png_rows(away)
            _w, _h, right = V.decode_png_rows(hover_png)
            hover_diff = V.pixel_changes(left, right)
        except (OSError, ValueError):
            hover_diff = 0
    exit_press = tool_run(binary, ["press-zoom", str(pid)])
    leave = burst(tool, pid, frames, "exit", mouse, None, n=5)
    restored_body = wait_chrome(lambda body: body.get("fullscreen") is False, 10)
    deadline = time.time() + 8
    while time.time() < deadline:
        live = V.read_layout_frames() or {}
        window = live.get("window")
        back = isinstance(window, list) and len(window) > 0 and abs(float(window[0]) - 1440) < 40
        if restored_body.get("fullscreen") is False and back:
            break
        restored_body = chrome_body()
        time.sleep(0.2)
    time.sleep(0.4)
    restored, restored_err = read_lights(binary, pid)
    copy_spike("chrome.json", "chrome-restored.json")
    copy_spike("shell-layout.json", "shell-layout-restored.json")
    copy_spike("layout.json", "layout-restored.json")
    metrics = V.window_metrics(tool, pid) or metrics
    if metrics.get("id"):
        V.capture(metrics["id"], OUT / "fullscreen-restored.png")
    after_inset = inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
    delta = frame_delta(windowed, restored)
    gaps = chrome_gaps(restored_body if restored_body.get("fullscreen") is False else chrome_body())
    windowed_gaps = chrome_gaps(before_chrome)
    inset_ok = False
    inset_nums = after_inset or {}
    if after_inset:
        inset_ok = all(abs(float(after_inset[key]) - 6.0) <= 0.5 for key in ("left", "top", "right", "bottom"))
    gap_ok = False
    if gaps and gaps.get("right") is not None:
        gap_ok = abs(gaps["left"] - 9.0) <= 0.5 and abs(gaps["right"] - 9.0) <= 0.5
    flicker_ok = enter["flips"] <= 1 and hover["flips"] <= 1 and leave["flips"] <= 1
    lights_shown = hover_diff > 0 or (hover_lights is not None and away_lights is None)
    delta_ok = delta is not None and delta <= 0.5
    ok = lights_shown and delta_ok and inset_ok and gap_ok and flicker_ok and restored is not None
    detail = (
        f"hover diff {hover_diff} frame delta {delta} inset {inset_nums} gaps {gaps} "
        f"enter flips {enter['flips']} hover flips {hover['flips']} exit flips {leave['flips']} "
        f"press {pressed.returncode}/{exit_press.returncode} "
        f"lights windowed {windowed_err or 'ok'} away {away_err or 'ok'} "
        f"hover {hover_err or 'ok'} restored {restored_err or 'ok'}"
    )
    numbers = {
        "hover_capture_diff": hover_diff,
        "frame_delta": delta,
        "inset": after_inset,
        "inset_before": before_inset,
        "gaps_restored_chrome": gaps,
        "gaps_windowed_chrome": windowed_gaps,
        "ax_windowed": windowed,
        "ax_away": away_lights,
        "ax_hover": hover_lights,
        "ax_restored": restored,
        "ax_gaps_restored": light_gaps(restored),
        "enter": enter,
        "hover": hover,
        "exit": leave,
        "fullscreen_flag": full.get("fullscreen"),
        "restored_flag": restored_body.get("fullscreen"),
    }
    (OUT / "fullscreen-numbers.json").write_text(json.dumps(numbers, indent=2) + "\n")
    return [row("fullscreen-lights", "pass" if ok else "fail", detail, "fullscreen-hover.png", numbers)]


def finish(rows, before, code):
    finish_held()
    after = snapshot_support(REAL_SUPPORT)
    (OUT / "support-after.json").write_text(json.dumps(after, indent=2) + "\n")
    changed = support_delta(before, after)
    support_ok = not changed
    rows.append(
        V.check(
            "real-support-untouched",
            "shell",
            "support",
            "pass" if support_ok else "fail",
            "real support dir unchanged" if support_ok else "changed " + ", ".join(changed[:12]),
            "support-before.json",
        )
    )
    rows[-1]["numbers"] = {"changed": changed, "before": len(before), "after": len(after)}
    say(rows[-1]["status"] + " real-support-untouched " + rows[-1]["detail"])
    head = subprocess.run(["git", "rev-parse", "HEAD"], cwd=WT, capture_output=True, text=True).stdout.strip()
    branch = subprocess.run(["git", "branch", "--show-current"], cwd=WT, capture_output=True, text=True).stdout.strip()
    failing = [item["id"] for item in rows if item["status"] == "fail"]
    summary = {
        "ok": not failing,
        "head": head,
        "branch": branch,
        "pr": "https://github.com/realmorrisliu/slowpage/pull/53",
        "nsh": V.nsh_version(),
        "cargo": V.cargo_bin(),
        "load_start": float(os.environ.get("SR_VERIFY_LOAD_START", "999")),
        "load_measured": V.one_minute_load(),
        "load_end": V.one_minute_load(),
        "load_max": float(os.environ.get("SR_VERIFY_MAX_LOAD", "999")),
        "support_dir": os.environ.get("SUPERREADER_SUPPORT_DIR", ""),
        "real_support_changed": changed,
        "checks": rows,
    }
    (OUT / "summary.json").write_text(json.dumps(summary, indent=2) + "\n")
    say(f"summary ok={summary['ok']} fails={failing}")
    return 0 if code is None and summary["ok"] else 1


if __name__ == "__main__":
    try:
        sys.exit(main())
    finally:
        finish_held()
