#!/usr/bin/env python3
"""Leave full screen and record the restored traffic-light frames.

The hover pass already showed the lights. AX could not see the fullscreen
window, so the zoom press that should leave full screen returned "no window".
This pass clicks the green light, then sends Ctrl-Command-F if the window is
still full screen.
"""

from __future__ import annotations

import json
import os
import subprocess
import time
from pathlib import Path

import drive
import retry

OUT = drive.OUT
V = drive.V


def main():
    os.chdir(drive.WT)
    os.environ["PATH"] = str(Path.home() / ".local" / "bin") + os.pathsep + os.environ.get("PATH", "")
    os.environ.pop("CARGO_TARGET_DIR", None)
    before = drive.snapshot_support(drive.REAL_SUPPORT)
    binary = retry.compile_retry_tools()
    lights = drive.compile_tools()
    window_tool = V.compile_window_id(OUT)
    root = OUT / "fixture-root"
    libs = V.FormLibraries(root)
    support = root / "support"
    os.environ["SUPERREADER_SUPPORT_DIR"] = str(support)
    spike = Path("/tmp/superreader-spike")
    spike.mkdir(parents=True, exist_ok=True)
    support_file = spike / "support-dir"
    previous = support_file.read_text() if support_file.is_file() else None
    support_file.write_text(str(support))
    numbers = {}
    try:
        state = drive.launch_form("exit-1440", root, libs, {"SPIKE_WINDOW": "1440x900"}, drive.titles_ready)
        pid = state["pid"]
        metrics, err = retry.capture_ok(window_tool, pid, OUT / "exit-windowed.png")
        print("windowed", err or "ok", None if not metrics else [metrics.get("w"), metrics.get("h")], flush=True)
        windowed, windowed_err = drive.read_lights(lights, pid)
        before_chrome = drive.chrome_body()
        before_inset = drive.inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
        pressed = retry.tool(lights, ["press-zoom", str(pid)])
        print("enter", pressed.returncode, (pressed.stderr or "").strip(), flush=True)
        full = drive.wait_chrome(lambda body: body.get("fullscreen") is True, 10)
        time.sleep(0.5)
        metrics = V.window_metrics(window_tool, pid) or metrics
        mouse = V.compile_mouse(OUT)
        hover_body = {}
        hover = drive.hover_point(metrics)
        for dy in (8.0, 20.0, 28.0, 40.0):
            point = (metrics["x"] + 36.0, metrics["y"] + dy)
            drive.move(mouse, *point)
            time.sleep(0.45)
            hover_body = drive.chrome_body()
            print("hover", dy, "lights", hover_body.get("lights"), flush=True)
            if float(hover_body.get("lights") or 0) >= 0.5:
                hover = point
                break
        print("window origin", metrics.get("x"), metrics.get("y"), metrics.get("w"), metrics.get("h"), flush=True)
        # Re-show the lights, then click the green disc without parking the pointer off the band.
        clicks = []
        for dx, dy in ((60.0, 24.0), (60.0, 32.0), (60.0, 40.0)):
            drive.move(mouse, metrics["x"] + 36.0, metrics["y"] + 40.0)
            time.sleep(0.35)
            shown = float(drive.chrome_body().get("lights") or 0)
            point = (metrics["x"] + dx, metrics["y"] + dy)
            retry.tool(binary, ["click", f"{point[0]:.1f}", f"{point[1]:.1f}"])
            time.sleep(0.45)
            flag = drive.chrome_body().get("fullscreen")
            clicks.append({"at": [dx, dy], "lights": shown, "fullscreen": flag})
            print("click", dx, dy, "lights", shown, "fullscreen", flag, flush=True)
            if flag is False:
                break
        click = (metrics["x"] + 60.0, metrics["y"] + 24.0)
        frames = []
        for index in range(5):
            path = OUT / "frames" / f"exit-click-{index:02d}.png"
            shot, shot_err = retry.capture_ok(window_tool, pid, path)
            height = float((shot or {}).get("h") or 0)
            count = drive.saturated_top(path, height) if path.is_file() and height else 0
            frames.append(
                {
                    "file": path.name,
                    "saturated": count,
                    "error": shot_err,
                    "size": None if not shot else [shot.get("w"), shot.get("h")],
                }
            )
            time.sleep(0.12)
        still = drive.chrome_body()
        key = None
        if still.get("fullscreen") is True:
            script = f'''
tell application "System Events"
  set frontmost of (first process whose unix id is {pid}) to true
  keystroke "f" using {{control down, command down}}
end tell
'''
            key = subprocess.run(["osascript", "-e", script], capture_output=True, text=True)
            print("key", key.returncode, (key.stderr or key.stdout or "").strip(), flush=True)
            time.sleep(0.4)
            for index in range(5):
                path = OUT / "frames" / f"exit-key-{index:02d}.png"
                shot, shot_err = retry.capture_ok(window_tool, pid, path)
                height = float((shot or {}).get("h") or 0)
                count = drive.saturated_top(path, height) if path.is_file() and height else 0
                frames.append(
                    {
                        "file": path.name,
                        "saturated": count,
                        "error": shot_err,
                        "size": None if not shot else [shot.get("w"), shot.get("h")],
                    }
                )
                time.sleep(0.12)
        deadline = time.time() + 8
        restored = drive.chrome_body()
        while time.time() < deadline:
            live = V.read_layout_frames() or {}
            window = live.get("window")
            back = isinstance(window, list) and window and abs(float(window[0]) - 1440) < 80
            if restored.get("fullscreen") is False and back:
                break
            time.sleep(0.2)
            restored = drive.chrome_body()
        time.sleep(0.5)
        restored_lights, restored_err = drive.read_lights(lights, pid)
        drive.copy_spike("chrome.json", "chrome-exit-restored.json")
        drive.copy_spike("shell-layout.json", "shell-layout-exit-restored.json")
        drive.copy_spike("layout.json", "layout-exit-restored.json")
        shot, shot_err = retry.capture_ok(window_tool, pid, OUT / "fullscreen-restored.png")
        after_inset = drive.inset_of(V.read_shell_layout(), V.read_layout_frames() or {})
        numbers = {
            "windowed_ax": windowed,
            "windowed_err": windowed_err,
            "windowed_gaps": drive.chrome_gaps(before_chrome),
            "inset_before": before_inset,
            "enter": pressed.returncode,
            "fullscreen": full.get("fullscreen"),
            "hover_lights": hover_body.get("lights"),
            "click": [round(click[0], 1), round(click[1], 1)],
            "clicks": clicks,
            "key": None if key is None else key.returncode,
            "frames": frames,
            "restored_fullscreen": restored.get("fullscreen"),
            "restored_ax": restored_lights,
            "restored_err": restored_err,
            "restored_gaps": drive.chrome_gaps(restored),
            "inset_after": after_inset,
            "frame_delta": drive.frame_delta(windowed, restored_lights),
            "restore_capture": shot_err,
            "restore_size": None if not shot else [shot.get("w"), shot.get("h")],
        }
        (OUT / "exit-numbers.json").write_text(json.dumps(numbers, indent=2) + "\n")
        print("delta", numbers["frame_delta"], "gaps", numbers["restored_gaps"], "inset", after_inset, "flag", restored.get("fullscreen"), flush=True)
    finally:
        drive.finish_held()
        if previous is None:
            support_file.unlink(missing_ok=True)
        else:
            support_file.write_text(previous)
        after = drive.snapshot_support(drive.REAL_SUPPORT)
        changed = drive.support_delta(before, after)
        print("support", changed, flush=True)


if __name__ == "__main__":
    main()
