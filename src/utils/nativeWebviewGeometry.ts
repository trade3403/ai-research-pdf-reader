/**
 * Convert the CSS viewport of the trusted Tauri main webview to physical
 * pixels for a separately hosted native WebView2 child window.
 *
 * CSS getBoundingClientRect() is expressed in CSS pixels. A native child
 * WebView2 must be positioned in physical pixels; mixing the two produces a
 * floating chat page at 2/3 scale on Windows 150% DPI.
 */
export type CssViewportRect = { left: number; top: number; width: number; height: number };
export type NativePhysicalRect = { x: number; y: number; width: number; height: number };
export type MeasuredPhysicalRect = NativePhysicalRect;

export function toPhysicalWebviewRect(rect: CssViewportRect, devicePixelRatio: number): NativePhysicalRect {
  if (!Number.isFinite(devicePixelRatio) || devicePixelRatio <= 0) {
    throw new Error('Invalid screen pixel ratio');
  }
  return {
    x: Math.max(0, Math.round(rect.left * devicePixelRatio)),
    y: Math.max(0, Math.round(rect.top * devicePixelRatio)),
    width: Math.max(1, Math.round(rect.width * devicePixelRatio)),
    height: Math.max(1, Math.round(rect.height * devicePixelRatio)),
  };
}

/**
 * Child Webview.position() reports parent-window-relative physical coordinates.
 * Compare them directly against setPosition() coordinates. Never subtract the
 * main window client origin (which incorrectly shifts y by the titlebar height).
 * A native viewport is revealed only after this read-back passes.
 */
export function matchesPhysicalWebviewRect(
  expected: NativePhysicalRect,
  measured: MeasuredPhysicalRect,
  tolerancePixels = 5,
): boolean {
  return (['x', 'y', 'width', 'height'] as const).every(
    key => Number.isFinite(measured[key]) && Math.abs(expected[key] - measured[key]) <= tolerancePixels,
  );
}

export function physicalWebviewGeometryKey(rect: NativePhysicalRect): string {
  return [rect.x, rect.y, rect.width, rect.height].join(':');
}
