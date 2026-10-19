/** 终端周边控件共用的透明状态层，保留原色的透明通道。 */
export function colorOverlay(color: string, percent: number): string {
  return `color-mix(in srgb, ${color} ${percent}%, transparent)`;
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
