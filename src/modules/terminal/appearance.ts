type Rgba = readonly [number, number, number, number];

// 方案入口已限制为六位或八位十六进制颜色。
function parseColor(color: string): Rgba {
  return [
    parseInt(color.slice(1, 3), 16),
    parseInt(color.slice(3, 5), 16),
    parseInt(color.slice(5, 7), 16),
    color.length === 9 ? parseInt(color.slice(7, 9), 16) / 255 : 1,
  ];
}

function rgba([r, g, b, alpha]: Rgba): string {
  return `rgba(${Math.round(r)}, ${Math.round(g)}, ${Math.round(b)}, ${alpha})`;
}

function weight(percent: number): number {
  return Math.min(100, Math.max(0, percent)) / 100;
}

/** 终端周边控件共用的透明状态层，保留原色的透明通道。 */
export function colorOverlay(color: string, percent: number): string {
  const [r, g, b, alpha] = parseColor(color);
  return rgba([r, g, b, alpha * weight(percent)]);
}

/** 按预乘透明度混合 sRGB，向控件库提供可解析的具体颜色值。 */
export function mixColors(
  color: string,
  background: string,
  percent: number,
): string {
  const front = parseColor(color);
  const back = parseColor(background);
  const ratio = weight(percent);
  const frontAlpha = front[3] * ratio;
  const backAlpha = back[3] * (1 - ratio);
  const alpha = frontAlpha + backAlpha;
  if (alpha === 0) return rgba([0, 0, 0, 0]);
  return rgba([
    (front[0] * frontAlpha + back[0] * backAlpha) / alpha,
    (front[1] * frontAlpha + back[1] * backAlpha) / alpha,
    (front[2] * frontAlpha + back[2] * backAlpha) / alpha,
    alpha,
  ]);
}

/** 仅决定局部控件表面的明暗，不改变终端或应用配色。 */
export function isLightBackground(background: string): boolean {
  const channels = [1, 3, 5].map((offset) => {
    const value = parseInt(background.slice(offset, offset + 2), 16) / 255;
    return value <= 0.04045 ? value / 12.92 : ((value + 0.055) / 1.055) ** 2.4;
  });
  return (
    channels[0]! * 0.2126 + channels[1]! * 0.7152 + channels[2]! * 0.0722 >
    0.179
  );
}
