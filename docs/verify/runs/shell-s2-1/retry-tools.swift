import AppKit
import ApplicationServices
import CoreGraphics

let args = CommandLine.arguments
guard args.count > 1 else { exit(2) }
let cmd = args[1]

func postKey(_ code: CGKeyCode, _ pid: pid_t?) {
  let src = CGEventSource(stateID: .hidSystemState)
  guard let down = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: true),
        let up = CGEvent(keyboardEventSource: src, virtualKey: code, keyDown: false) else { return }
  if let pid {
    down.postToPid(pid)
    up.postToPid(pid)
  } else {
    down.post(tap: .cghidEventTap)
    up.post(tap: .cghidEventTap)
  }
}

if cmd == "move" || cmd == "click" {
  guard args.count > 3, let x = Double(args[2]), let y = Double(args[3]) else { exit(2) }
  let point = CGPoint(x: x, y: y)
  if cmd == "move" {
    CGWarpMouseCursorPosition(point)
    let src = CGEventSource(stateID: .hidSystemState)
    if let event = CGEvent(mouseEventSource: src, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left) {
      event.post(tap: .cghidEventTap)
    }
  } else {
    let src = CGEventSource(stateID: .hidSystemState)
    let down = CGEvent(mouseEventSource: src, mouseType: .leftMouseDown, mouseCursorPosition: point, mouseButton: .left)
    let up = CGEvent(mouseEventSource: src, mouseType: .leftMouseUp, mouseCursorPosition: point, mouseButton: .left)
    down?.post(tap: .cghidEventTap)
    up?.post(tap: .cghidEventTap)
  }
  exit(0)
}

if cmd == "wheel" {
  guard args.count > 4, let x = Double(args[2]), let y = Double(args[3]), let dy = Int32(args[4]) else { exit(2) }
  let point = CGPoint(x: x, y: y)
  CGWarpMouseCursorPosition(point)
  guard let event = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: dy, wheel2: 0, wheel3: 0) else { exit(1) }
  event.location = point
  event.post(tap: .cghidEventTap)
  exit(0)
}

if cmd == "pagedown" {
  let pid = args.count > 2 ? pid_t(args[2]) : nil
  postKey(121, pid)
  exit(0)
}

if cmd == "exit-fullscreen" {
  guard args.count > 2, let pid = pid_t(args[2]) else { exit(2) }
  let src = CGEventSource(stateID: .hidSystemState)
  guard let down = CGEvent(keyboardEventSource: src, virtualKey: 3, keyDown: true),
        let up = CGEvent(keyboardEventSource: src, virtualKey: 3, keyDown: false) else { exit(1) }
  down.flags = [.maskCommand, .maskControl]
  up.flags = [.maskCommand, .maskControl]
  down.postToPid(pid)
  up.postToPid(pid)
  exit(0)
}

func axValue(_ el: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(el, name as CFString, &value) == .success else { return nil }
  return value
}

func walk(_ el: AXUIElement, _ depth: Int, _ visit: (AXUIElement) -> Void) {
  if depth > 18 { return }
  visit(el)
  var kids: CFTypeRef?
  if AXUIElementCopyAttributeValue(el, kAXChildrenAttribute as CFString, &kids) == .success,
     let kids = kids as? [AXUIElement] {
    for kid in kids { walk(kid, depth + 1, visit) }
  }
}

func label(of el: AXUIElement) -> String {
  let title = axValue(el, kAXTitleAttribute as String) as? String ?? ""
  if !title.isEmpty && title != "missing value" { return title }
  let desc = axValue(el, kAXDescriptionAttribute as String) as? String ?? ""
  if !desc.isEmpty && desc != "missing value" { return desc }
  return ""
}

guard args.count > 2, let pid = pid_t(args[2]) else { exit(2) }
let app = AXUIElementCreateApplication(pid)
var raw: CFTypeRef?
guard AXUIElementCopyAttributeValue(app, kAXWindowsAttribute as CFString, &raw) == .success,
      let windows = raw as? [AXUIElement] else {
  fputs("no window\n", stderr)
  exit(1)
}

if cmd == "buttons" {
  for window in windows {
    walk(window, 0) { el in
      let role = axValue(el, kAXRoleAttribute as String) as? String ?? ""
      let name = label(of: el)
      if role == "AXButton" && !name.isEmpty {
        print(name)
      }
    }
  }
  exit(0)
}

if cmd == "press" {
  guard args.count > 3 else { exit(2) }
  let want = args[3]
  for window in windows {
    var pressed = false
    walk(window, 0) { el in
      if pressed { return }
      let role = axValue(el, kAXRoleAttribute as String) as? String ?? ""
      if role == "AXButton" && label(of: el) == want {
        let err = AXUIElementPerformAction(el, kAXPressAction as CFString)
        if err == .success {
          print("pressed")
          pressed = true
        } else {
          fputs("press \(err.rawValue)\n", stderr)
        }
      }
    }
    if pressed { exit(0) }
  }
  fputs("missing \(want)\n", stderr)
  exit(1)
}

fputs("unknown \(cmd)\n", stderr)
exit(2)
