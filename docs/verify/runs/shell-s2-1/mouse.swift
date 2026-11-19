
import CoreGraphics
guard CommandLine.arguments.count > 2,
      let x = Double(CommandLine.arguments[1]),
      let y = Double(CommandLine.arguments[2]) else {
  exit(2)
}
let point = CGPoint(x: x, y: y)
guard let event = CGEvent(mouseEventSource: nil, mouseType: .mouseMoved, mouseCursorPosition: point, mouseButton: .left) else {
  exit(1)
}
event.post(tap: .cghidEventTap)
