"""Pack assets/*.pwevid files into a raw IDE disk image for PweOS."""

from __future__ import annotations

import argparse
import struct
from pathlib import Path

SECTOR_SIZE = 512
MAX_FILES = (SECTOR_SIZE - 12) // 40


def pack(asset_dir: Path, output: Path) -> None:
    videos = sorted(asset_dir.glob("*.pwevid"))
    if len(videos) > MAX_FILES:
        raise SystemExit(f"at most {MAX_FILES} video files fit in the PweOS disk index")

    entries: list[tuple[Path, int, bytes]] = []
    next_sector = 1
    for video in videos:
        data = video.read_bytes()
        if len(video.name.encode("utf-8")) >= 32:
            raise SystemExit(f"video filename must be shorter than 32 bytes: {video.name}")
        if not data.startswith(b"PWEV"):
            raise SystemExit(f"not a PweOS video file: {video}")
        entries.append((video, next_sector, data))
        next_sector += (len(data) + SECTOR_SIZE - 1) // SECTOR_SIZE

    image = bytearray(next_sector * SECTOR_SIZE)
    image[:12] = b"PWEOSVID" + struct.pack("<HH", 1, len(entries))
    for index, (video, sector, data) in enumerate(entries):
        offset = 12 + index * 40
        encoded_name = video.name.encode("utf-8")
        image[offset : offset + len(encoded_name)] = encoded_name
        struct.pack_into("<II", image, offset + 32, sector, len(data))
        begin = sector * SECTOR_SIZE
        image[begin : begin + len(data)] = data

    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_bytes(image)
    print(f"Packed {len(entries)} video(s) into {output} ({len(image):,} bytes)")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("asset_dir", type=Path)
    parser.add_argument("output", type=Path)
    args = parser.parse_args()
    pack(args.asset_dir.resolve(), args.output.resolve())


if __name__ == "__main__":
    main()
