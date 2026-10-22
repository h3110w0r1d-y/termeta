"""从确认的品牌 SVG 生成桌面图标；不编译、启动或打包项目。"""

import argparse
from io import BytesIO
import hashlib
from importlib.metadata import version
import json
from pathlib import Path
import shutil
import struct
import subprocess
import tempfile
import xml.etree.ElementTree as ET

from PIL import Image, ImageCms, ImageDraw
import resvg_py


ROOT = Path(__file__).resolve().parent.parent
NS = "http://www.w3.org/2000/svg"
ET.register_namespace("", NS)
PROFILE = ImageCms.ImageCmsProfile(ImageCms.createProfile("sRGB")).tobytes()


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def master_svg(macos, source_path):
    tree = ET.parse(source_path)
    root = tree.getroot()
    # 只调整取景，不改动已确认图稿的形体、层间关系、渐变或阴影。
    # 通用版去掉外部空白；macOS 版底板宽度占 824/1024，并等比居中。
    side = 1008 * 1024 / 824 if macos else 1008
    box = [626 - side / 2, 602.5 - side / 2, side, side]
    root.set("viewBox", " ".join(f"{value:.9f}" for value in box))
    root.set("width", "2048")
    root.set("height", "2048")
    return ET.tostring(root, encoding="unicode")


def render_png(source, size, output):
    image = Image.open(BytesIO(resvg_py.svg_to_bytes(
        svg_string=source, width=size, height=size,
    ))).convert("RGBA")
    image.save(output, icc_profile=PROFILE)


def ico_entries(data):
    reserved, kind, count = struct.unpack_from("<HHH", data)
    assert (reserved, kind) == (0, 1) and count > 0
    entries = []
    for i in range(count):
        fields = struct.unpack_from("<BBBBHHII", data, 6 + 16 * i)
        w, h, colors, zero, planes, bits, size, offset = fields
        assert offset >= 6 + 16 * count and offset + size <= len(data)
        entries.append({"size": (w or 256, h or 256),
                        "fields": fields, "payload": data[offset:offset + size]})
    return entries


def write_ico(source, path):
    # 从矢量逐尺寸渲染，避免大 PNG 再缩小时在透明圆角产生振铃。
    # 显式写入 ICO 目录顺序，保证默认读取第一项的窗口也获得高清图。
    sizes = [256, 128, 64, 48, 32, 24, 16]
    header = bytearray(struct.pack("<HHH", 0, 1, len(sizes)))
    offset = 6 + 16 * len(sizes)
    payloads = []
    for size in sizes:
        image = Image.open(BytesIO(resvg_py.svg_to_bytes(
            svg_string=source, width=size, height=size,
        ))).convert("RGBA")
        buffer = BytesIO()
        image.save(buffer, format="PNG", icc_profile=PROFILE)
        payload = buffer.getvalue()
        dimension = 0 if size == 256 else size
        header.extend(struct.pack("<BBBBHHII", dimension, dimension, 0, 0,
                                  1, 32, len(payload), offset))
        payloads.append(payload)
        offset += len(payload)
    path.write_bytes(header + b"".join(payloads))
    return [(size, size) for size in sizes]


def tauri_icons(master, output):
    result = subprocess.run(
        [shutil.which("pnpm") or "pnpm", "exec", "tauri", "icon",
         str(master), "--output", str(output)],
        cwd=ROOT, text=True, capture_output=True,
    )
    if result.returncode:
        raise RuntimeError(result.stdout + result.stderr)


def verify_icons(directory):
    for path in directory.glob("*.png"):
        with Image.open(path) as image:
            assert image.mode == "RGBA" and image.width == image.height, path
            assert image.getpixel((0, 0))[3] == 0, path
    ico = Image.open(directory / "icon.ico")
    entries = ico_entries((directory / "icon.ico").read_bytes())
    assert entries[0]["size"] == (256, 256)
    assert entries[0]["fields"][5] == 32
    for entry in entries:
        decoded = ico.ico.getimage(entry["size"]).convert("RGBA")
        assert decoded.size == entry["size"]
        assert decoded.getpixel((0, 0))[3] == 0
    icns = Image.open(directory / "icon.icns")
    # ICNS 的 512pt@2x 也对应 1024 像素，不能只认 1024pt@1x。
    assert max(w * density for w, h, density in icns.info["sizes"]) == 1024
    icns.load()
    assert icns.size == (1024, 1024)
    assert icns.getpixel((0, 0))[3] == 0
    config = json.loads((ROOT / "src-tauri/tauri.conf.json").read_text())
    for icon in config["bundle"]["icon"]:
        assert (directory / Path(icon).name).is_file(), icon


def generate(artifacts, theme):
    artifacts.mkdir(parents=True, exist_ok=True)
    icon_name = "termeta-icon-light.svg" if theme == "light" else "termeta-icon.svg"
    source_path = ROOT / "resources/brand" / icon_name
    destination = ROOT / "src-tauri/icons"
    favicon = ROOT / "public/terminal.svg"
    if theme == "light":
        destination /= "light"
        favicon = ROOT / "public/terminal-light.svg"
    for name in [icon_name, "termeta-symbol.svg"]:
        shutil.copy2(ROOT / "resources/brand" / name, artifacts / name)
    masters = artifacts / "masters"
    masters.mkdir(exist_ok=True)
    source_hash = digest(source_path)
    sources = {"standard": master_svg(False, source_path), "macos": master_svg(True, source_path)}
    # 平台母图仅改变根画布，内部元素必须与终版完全一致。
    approved = ET.parse(source_path).getroot()
    for name, source in sources.items():
        variant = ET.fromstring(source)
        assert [ET.tostring(e) for e in approved] == [ET.tostring(e) for e in variant]
        (masters / f"app-icon-{name}.svg").write_text(source + "\n")
        for size in [1024, 2048]:
            render_png(source, size, masters / f"app-icon-{name}-{size}.png")

    with tempfile.TemporaryDirectory(prefix="termeta-icons-") as temp:
        staging = Path(temp)
        tauri_icons(masters / "app-icon-standard-2048.png", staging / "standard")
        tauri_icons(masters / "app-icon-macos-2048.png", staging / "macos")
        generated = artifacts / "icons"
        generated.mkdir(exist_ok=True)
        # Tauri 同时生成移动端素材；仅接受桌面根目录的资源。
        for path in (staging / "standard").iterdir():
            if path.is_file() and path.suffix in {".png", ".ico"}:
                shutil.copy2(path, generated / path.name)
        shutil.copy2(staging / "macos/icon.icns", generated / "icon.icns")
        order = write_ico(sources["standard"], generated / "icon.ico")
        verify_icons(generated)
        destination.mkdir(exist_ok=True)
        for path in generated.iterdir():
            if path.is_file():
                shutil.copy2(path, destination / path.name)

    favicon.write_text(sources["standard"] + "\n")
    verify_icons(destination)
    assert digest(source_path) == source_hash
    preview = Image.new("RGB", (1024, 570), "#101827")
    draw = ImageDraw.Draw(preview)
    for i, name in enumerate(["standard", "macos"]):
        image = Image.open(masters / f"app-icon-{name}-1024.png").resize((480, 480))
        preview.paste(image, (16 + i * 512, 45), image)
        draw.text((24 + i * 512, 18), name.upper(), fill="#bfd0e5")
    preview.save(artifacts / "preview.png")
    manifest = {
        "theme": theme,
        "source": str(source_path.relative_to(ROOT)), "source_sha256": source_hash,
        "tools": {"tauri_cli": json.loads((ROOT / "node_modules/@tauri-apps/cli/package.json").read_text())["version"],
                  "Pillow": version("Pillow"), "resvg-py": version("resvg-py")},
        "ico_order": order, "ico_first_bit_depth": 32,
        "macos_max_size": [1024, 1024], "macos_tile_width_fraction": 824 / 1024,
        "files": {str(path.relative_to(ROOT)): digest(path)
                  for path in sorted(destination.iterdir()) if path.is_file()},
        "favicon_sha256": digest(favicon),
        "validation": "已静态解码各尺寸与检查透明通道；未构建、启动或进行系统展示验收",
    }
    (artifacts / "manifest.json").write_text(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
    print(json.dumps({"theme": theme, "icons": len(manifest["files"]), "ico_order": order,
                      "artifacts": str(artifacts)}, ensure_ascii=False))


def generate_themes(artifacts, theme):
    if theme in {"dark", "all"}:
        generate(artifacts, "dark")
    if theme in {"light", "all"}:
        generate(artifacts / "light" if theme == "all" else artifacts, "light")
    if theme == "all":
        preview = Image.new("RGB", (1024, 570), "#9babc1")
        draw = ImageDraw.Draw(preview)
        for i, (folder, label) in enumerate([(artifacts, "DARK"), (artifacts / "light", "LIGHT")]):
            image = Image.open(folder / "masters/app-icon-macos-1024.png").resize((512, 512))
            preview.paste(image, (i * 512, 45), image)
            draw.text((24 + i * 512, 18), label, fill="#14243a")
        preview.save(artifacts / "theme-comparison.png")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--artifacts-dir", type=Path, help="可选的母图、预览和生成报告输出目录")
    parser.add_argument("--theme", choices=["dark", "light", "all"], default="all", help="默认生成深浅两套；项目默认入口使用深色版")
    args = parser.parse_args()
    if args.artifacts_dir:
        generate_themes(args.artifacts_dir.resolve(), args.theme)
    else:
        with tempfile.TemporaryDirectory(prefix="termeta-icon-artifacts-") as temp:
            generate_themes(Path(temp), args.theme)
