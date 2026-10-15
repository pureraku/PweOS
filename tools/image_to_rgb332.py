# /// script
# requires-python = ">=3.11"
# dependencies = ["Pillow>=10.0.0"]
# ///
"""Convert an image to PweOS's 320x200 raw RGB332 format."""

from __future__ import annotations

import argparse
from pathlib import Path

from PIL import Image, ImageOps


WIDTH = 320
HEIGHT = 200
PROJECT_ROOT = Path(__file__).resolve().parents[1]


def convert(source: Path, destination: Path) -> None:
    with Image.open(source) as image:
        image = ImageOps.exif_transpose(image).convert("RGB")
        fitted = ImageOps.contain(
            image,
            (WIDTH, HEIGHT),
            method=Image.Resampling.LANCZOS,
        )
        canvas = Image.new("RGB", (WIDTH, HEIGHT), (0, 0, 0))
        canvas.paste(fitted, ((WIDTH - fitted.width) // 2, (HEIGHT - fitted.height) // 2))

    rgb = canvas.tobytes()
    rgb332 = bytearray(WIDTH * HEIGHT)
    for pixel_index, offset in enumerate(range(0, len(rgb), 3)):
        red, green, blue = rgb[offset : offset + 3]
        rgb332[pixel_index] = (
            (((red * 7 + 127) // 255) << 5)
            | (((green * 7 + 127) // 255) << 2)
            | ((blue * 3 + 127) // 255)
        )

    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(rgb332)


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("image", type=Path, help="source image, such as cat.jpg")
    parser.add_argument(
        "-o",
        "--output",
        type=Path,
        help="output .rgb332 path (default: assets/<source-name>.rgb332)",
    )
    args = parser.parse_args()

    source = args.image.expanduser().resolve()
    if not source.is_file():
        parser.error(f"image not found: {source}")
    destination = args.output or PROJECT_ROOT / "assets" / f"{source.stem}.rgb332"
    destination = destination.expanduser().resolve()
    if destination.suffix != ".rgb332":
        parser.error("output filename must end in .rgb332")
    if len(destination.name.encode("utf-8")) >= 32:
        parser.error("output filename must be shorter than 32 bytes for RAMFS")
    convert(source, destination)
    print(f"Wrote {WIDTH}x{HEIGHT} RGB332 image ({WIDTH * HEIGHT} bytes): {destination}")


if __name__ == "__main__":
    main()
