
import CoreGraphics
import Foundation
let args = CommandLine.arguments
guard args.count == 6,
      let x1 = Double(args[1]), let y1 = Double(args[2]),
      let x2 = Double(args[3]), let y2 = Double(args[4]),
      let duration = Double(args[5]) else { exit(2) }
func post(_ type: CGEventType, _ x: Double, _ y: Double) {
  guard let event = CGEvent(mouseEventSource: nil, mouseType: type, mouseCursorPosition: CGPoint(x: x, y: y), mouseButton: .left) else { return }
  event.setIntegerValueField(.mouseEventClickState, value: 1)
  event.post(tap: .cghidEventTap)
}
post(.leftMouseDown, x1, y1)
let steps = 40
let pause = useconds_t(max(1000, duration / Double(steps) * 1_000_000))
for step in 1...steps {
  let u = Double(step) / Double(steps)
  post(.leftMouseDragged, x1 + (x2 - x1) * u, y1 + (y2 - y1) * u)
  usleep(pause)
}
post(.leftMouseUp, x2, y2)
