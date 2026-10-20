import manifest from "@resources/host-system-icons.json";

export const systemIconOptions = manifest.map(({ id, label }) => ({
  value: id,
  label,
}));
const assets = import.meta.glob<string>("@/assets/svg/systems/*.svg", {
  base: "/src/assets/svg/systems",
  eager: true,
  query: "?raw",
  import: "default",
});

export function systemIconBackground(icon: string): string {
  return manifest.find(({ id }) => id === icon)?.background ?? "#004878";
}

// 白名单和静态资源来自同一份清单；远端字符串不能作为资源路径或HTML。
export function systemIconSvg(icon: string, prefix: string): string {
  const file = manifest.find(({ id }) => id === icon)?.file ?? "ssh.svg";
  const svg = assets[`./${file}`] ?? "";
  // 解包SVG使用短ID，给每个实例加命名空间，防止卡片和选择器的裁切路径串用。
  return svg
    .replace(/id="([^"]+)"/g, (_, id: string) => `id="${prefix}-${id}"`)
    .replace(/url\(#([^)]+)\)/g, (_, id: string) => `url(#${prefix}-${id})`)
    .replace(/(href=")#([^"]+)/g, "$1#" + prefix + "-$2");
}
