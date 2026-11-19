
import CoreGraphics
guard CommandLine.arguments.count > 3,
      let x = Double(CommandLine.arguments[1]),
      let y = Double(CommandLine.arguments[2]),
      let dy = Int32(CommandLine.arguments[3]) else { exit(2) }
let point = CGPoint(x: x, y: y)
CGWarpMouseCursorPosition(point)
guard let event = CGEvent(scrollWheelEvent2Source: nil, units: .pixel, wheelCount: 1, wheel1: dy, wheel2: 0, wheel3: 0) else { exit(1) }
event.location = point
event.post(tap: .cghidEventTap)
