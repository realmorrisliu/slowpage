
import ApplicationServices
import CoreGraphics
import Foundation
guard CommandLine.arguments.count > 1, let pid = pid_t(CommandLine.arguments[1]) else { exit(2) }
let app = AXUIElementCreateApplication(pid)
func ax(_ el: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(el, name as CFString, &value) == .success else { return nil }
  return value
}
func walk(_ el: AXUIElement, _ depth: Int, _ hit: (AXUIElement, String) -> Void) {
  if depth > 20 { return }
  let role = (ax(el, kAXRoleAttribute as String) as? String) ?? ""
  hit(el, role)
  if let kids = ax(el, kAXChildrenAttribute as String) as? [AXUIElement] {
    for kid in kids { walk(kid, depth + 1, hit) }
  }
}
var bars: [AXUIElement] = []
guard let windows = ax(app, kAXWindowsAttribute as String) as? [AXUIElement] else { exit(1) }
for window in windows {
  walk(window, 0) { el, role in
    if role == "AXScrollBar" { bars.append(el) }
  }
}
var moved = 0
for bar in bars {
  let orient = (ax(bar, "AXOrientation") as? String) ?? ""
  if orient != "AXVerticalOrientation" && !orient.isEmpty { continue }
  var maxValue: CFTypeRef?
  AXUIElementCopyAttributeValue(bar, kAXMaxValueAttribute as CFString, &maxValue)
  let maxNum = (maxValue as? NSNumber)?.doubleValue ?? 1
  let number = maxNum as CFTypeRef
  let err = AXUIElementSetAttributeValue(bar, kAXValueAttribute as CFString, number)
  if err == .success { moved += 1 }
  fputs("bar \(orient) max \(maxNum) err \(err.rawValue)\n", stderr)
}
print("moved \(moved) bars \(bars.count)")
