
import ApplicationServices
import Foundation
guard CommandLine.arguments.count > 1, let pid = pid_t(CommandLine.arguments[1]) else { exit(2) }
let app = AXUIElementCreateApplication(pid)
func ax(_ el: AXUIElement, _ name: String) -> CFTypeRef? {
  var value: CFTypeRef?
  guard AXUIElementCopyAttributeValue(el, name as CFString, &value) == .success else { return nil }
  return value
}
func box(_ el: AXUIElement) -> (CGFloat, CGFloat, CGFloat, CGFloat)? {
  guard let p = ax(el, kAXPositionAttribute as String),
        let s = ax(el, kAXSizeAttribute as String) else { return nil }
  var pos = CGPoint.zero
  var size = CGSize.zero
  guard AXValueGetValue(p as! AXValue, .cgPoint, &pos),
        AXValueGetValue(s as! AXValue, .cgSize, &size) else { return nil }
  return (pos.x, pos.y, size.width, size.height)
}
func walk(_ el: AXUIElement, _ depth: Int, _ origin: (CGFloat, CGFloat)) {
  if depth > 16 { return }
  let role = (ax(el, kAXRoleAttribute as String) as? String) ?? ""
  var name = (ax(el, kAXTitleAttribute as String) as? String) ?? ""
  if name.isEmpty { name = (ax(el, kAXDescriptionAttribute as String) as? String) ?? "" }
  if let frame = box(el), !name.isEmpty, frame.2 > 0, frame.3 > 0 {
    let x = frame.0 - origin.0
    let y = frame.1 - origin.1
    print("\(role)\t\(name)\t\(x)\t\(y)\t\(frame.2)\t\(frame.3)")
  }
  if let kids = ax(el, kAXChildrenAttribute as String) as? [AXUIElement] {
    for kid in kids { walk(kid, depth + 1, origin) }
  }
}
guard let windows = ax(app, kAXWindowsAttribute as String) as? [AXUIElement] else { exit(1) }
for window in windows {
  guard let frame = box(window) else { continue }
  walk(window, 0, (frame.0, frame.1))
}
