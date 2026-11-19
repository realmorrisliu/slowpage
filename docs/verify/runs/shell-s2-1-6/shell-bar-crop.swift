
import AppKit
import CoreText
import ImageIO
import UniformTypeIdentifiers
let args = CommandLine.arguments
guard args.count > 8,
      let windowW = Double(args[3]),
      let barW = Double(args[4]),
      let closeMin = Double(args[5]),
      let zoomMax = Double(args[6]),
      let leftGap = Double(args[7]),
      let rightGap = Double(args[8]) else { exit(2) }
let src = URL(fileURLWithPath: args[1])
let dst = URL(fileURLWithPath: args[2])
guard let data = try? Data(contentsOf: src),
      let source = CGImageSourceCreateWithData(data as CFData, nil),
      let image = CGImageSourceCreateImageAtIndex(source, 0, nil) else { exit(1) }
let pixelW = CGFloat(image.width)
let pixelH = CGFloat(image.height)
let backing = pixelW / CGFloat(windowW)
let cropW = min(CGFloat(barW) * backing, pixelW)
let cropH = min(72 * backing, pixelH)
guard let cropped = image.cropping(to: CGRect(x: 0, y: 0, width: cropW, height: cropH)) else { exit(1) }
let outW = Int(cropW * 2)
let outH = Int(cropH * 2)
guard let ctx = CGContext(data: nil, width: outW, height: outH, bitsPerComponent: 8, bytesPerRow: 0, space: CGColorSpaceCreateDeviceRGB(), bitmapInfo: CGImageAlphaInfo.premultipliedLast.rawValue) else { exit(1) }
ctx.interpolationQuality = .none
ctx.draw(cropped, in: CGRect(x: 0, y: 0, width: outW, height: outH))
let scale = backing * 2
func vx(_ pt: Double) -> CGFloat { CGFloat(pt) * scale }
ctx.setStrokeColor(CGColor(red: 0.75, green: 0.15, blue: 0.12, alpha: 1))
ctx.setLineWidth(2)
for x in [closeMin, zoomMax, barW] {
  ctx.move(to: CGPoint(x: vx(x), y: 0))
  ctx.addLine(to: CGPoint(x: vx(x), y: CGFloat(outH)))
}
ctx.strokePath()
func label(_ text: String, _ x: CGFloat, alignRight: Bool = false) {
  let font = CTFontCreateWithName("Iowan Old Style" as CFString, 22, nil)
  let attrs: [NSAttributedString.Key: Any] = [.font: font, .foregroundColor: NSColor(red: 0.75, green: 0.15, blue: 0.12, alpha: 1)]
  let line = CTLineCreateWithAttributedString(NSAttributedString(string: text, attributes: attrs))
  let textW = CGFloat(CTLineGetTypographicBounds(line, nil, nil, nil))
  var origin = x
  if alignRight {
    origin = min(x, CGFloat(outW) - textW - 8)
  }
  ctx.textPosition = CGPoint(x: max(origin, 4), y: 18)
  CTLineDraw(line, ctx)
}
label(String(format: "L %.1f", leftGap), vx(0) + 6)
label(String(format: "R %.1f", rightGap), vx(barW) - 8, alignRight: true)
guard let painted = ctx.makeImage(),
      let dest = CGImageDestinationCreateWithURL(dst as CFURL, UTType.png.identifier as CFString, 1, nil) else { exit(1) }
CGImageDestinationAddImage(dest, painted, nil)
if !CGImageDestinationFinalize(dest) { exit(1) }
