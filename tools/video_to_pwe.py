"""Convert a short video into PweOS's compact 320x200 RGB332 video format.

Requires ffmpeg on PATH. Output files belong in assets/ to bundle them into
the boot image and RAMFS.
"""

from __future__ import annotations

import argparse
import struct
import subprocess
from pathlib import Path

WIDTH = 320
HEIGHT = 200
PROJECT_ROOT = Path(__file__).resolve().parents[1]
PIXELS = WIDTH * HEIGHT


def read_exact(stream, size: int) -> bytes:
    chunks = bytearray()
    while len(chunks) < size:
        chunk = stream.read(size - len(chunks))
        if not chunk:
            break
        chunks.extend(chunk)
    return bytes(chunks)


def to_rgb332(rgb: bytes) -> bytes:
    packed = bytearray(PIXELS)
    for i in range(PIXELS):
        r, g, b = rgb[i * 3 : i * 3 + 3]
        packed[i] = ((r * 7 + 127) // 255 << 5) | ((g * 7 + 127) // 255 << 2) | ((b * 3 + 127) // 255)
    return bytes(packed)


def encode_keyframe(frame: bytes) -> bytes:
    encoded = bytearray()
    i = 0
    while i < PIXELS:
        color = frame[i]
        end = i + 1
        while end < PIXELS and frame[end] == color and end - i < 255:
            end += 1
        encoded.extend((end - i, color))
        i = end
    return bytes(encoded)


def encode_delta(previous: bytes, current: bytes) -> bytes:
    encoded = bytearray()
    i = 0
    while i < PIXELS:
        start = i
        while i < PIXELS and current[i] == previous[i]:
            i += 1
        skip = i - start
        start = i
        while i < PIXELS and current[i] != previous[i]:
            i += 1
        run = i - start
        encoded.extend(struct.pack("<HH", skip, run))
        encoded.extend(current[start:i])
        if run == 0:
            break
    return bytes(encoded)


def convert(source: Path, destination: Path, fps: int, seconds: float, keyframe_seconds: int) -> None:
    frame_limit = max(1, int(fps * seconds))
    keyframe_interval = max(1, fps * keyframe_seconds)
    command = [
        "ffmpeg", "-v", "error", "-i", str(source),
        "-vf", f"fps={fps},scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:black,setsar=1",
        "-frames:v", str(frame_limit), "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
    ]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert process.stdout is not None
    frames: list[bytes] = []
    while len(frames) < frame_limit:
        raw = read_exact(process.stdout, PIXELS * 3)
        if not raw:
            break
        if len(raw) != PIXELS * 3:
            process.kill()
            raise RuntimeError("ffmpeg returned an incomplete frame")
        frames.append(to_rgb332(raw))
    process.stdout.close()
    stderr = process.stderr.read() if process.stderr else b""
    status = process.wait()
    if status:
        raise RuntimeError(stderr.decode(errors="replace").strip() or "ffmpeg failed")
    if not frames:
        raise RuntimeError("video contained no decodable frames")

    output = bytearray(b"PWEV")
    output.extend(struct.pack("<BBHHH", 1, fps, len(frames), WIDTH, HEIGHT))
    previous = bytes(PIXELS)
    for index, frame in enumerate(frames):
        keyframe = index == 0 or index % keyframe_interval == 0
        encoded = encode_keyframe(frame) if keyframe else encode_delta(previous, frame)
        output.append(0 if keyframe else 1)
        output.extend(struct.pack("<I", len(encoded)))
        output.extend(encoded)
        previous = frame

    destination.parent.mkdir(parents=True, exist_ok=True)
    destination.write_bytes(output)
    print(f"Wrote {len(frames)} frames at {fps} FPS ({len(output):,} bytes): {destination}")
    if len(output) > 180 * 1024:
        print("Warning: large video assets may exceed the boot kernel's size limit; try a shorter clip or lower FPS.")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("video", type=Path, help="input video supported by ffmpeg")
    parser.add_argument("-o", "--output", type=Path, help="output file (default: assets/<name>.pwevid)")
    parser.add_argument("--fps", type=int, default=10, help="playback rate from 1 to 30 (default: 10)")
    parser.add_argument("--seconds", type=float, default=3.0, help="maximum clip length (default: 3 seconds)")
    parser.add_argument("--keyframe-seconds", type=int, default=2, help="full frame interval (default: 2 seconds)")
    args = parser.parse_args()
    source = args.video.expanduser().resolve()
    if not source.is_file():
        parser.error(f"video not found: {source}")
    if not 1 <= args.fps <= 30 or args.seconds <= 0 or args.keyframe_seconds <= 0:
        parser.error("FPS must be 1-30 and both durations must be positive")
    destination = args.output or PROJECT_ROOT / "assets" / f"{source.stem}.pwevid"
    destination = destination.expanduser().resolve()
    if destination.suffix != ".pwevid":
        parser.error("output filename must end in .pwevid")
    if len(destination.name.encode("utf-8")) >= 32:
        parser.error("output filename must be shorter than 32 bytes for RAMFS")
    try:
        convert(source, destination, args.fps, args.seconds, args.keyframe_seconds)
    except FileNotFoundError:
        parser.error("ffmpeg was not found; install ffmpeg and make sure it is on PATH")
    except RuntimeError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
