"""Convert a video into PweOS's compact 320x200 RGB332 video format.

Requires ffmpeg on PATH. Output files belong in assets/ and are packed into the
companion video disk when the OS is built.
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


def convert(source: Path, destination: Path, fps: int, seconds: float | None, keyframe_seconds: int) -> None:
    frame_limit = max(1, int(fps * seconds)) if seconds is not None else None
    keyframe_interval = max(1, fps * keyframe_seconds)
    command = [
        "ffmpeg", "-v", "error", "-i", str(source),
        "-vf", f"fps={fps},scale={WIDTH}:{HEIGHT}:force_original_aspect_ratio=decrease,pad={WIDTH}:{HEIGHT}:(ow-iw)/2:(oh-ih)/2:black,setsar=1",
        *( ["-frames:v", str(frame_limit)] if frame_limit is not None else [] ),
        "-f", "rawvideo", "-pix_fmt", "rgb24", "pipe:1",
    ]
    process = subprocess.Popen(command, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
    assert process.stdout is not None
    destination.parent.mkdir(parents=True, exist_ok=True)
    frame_count = 0
    previous = bytes(PIXELS)
    try:
        with destination.open("wb") as output:
            output.write(b"PWEV" + struct.pack("<BBHHH", 1, fps, 0, WIDTH, HEIGHT))
            while frame_limit is None or frame_count < frame_limit:
                raw = read_exact(process.stdout, PIXELS * 3)
                if not raw:
                    break
                if frame_count >= 65535:
                    process.kill()
                    raise RuntimeError("video exceeds the format limit of 65,535 frames; lower FPS or use --seconds")
                if len(raw) != PIXELS * 3:
                    process.kill()
                    raise RuntimeError("ffmpeg returned an incomplete frame")
                frame = to_rgb332(raw)
                keyframe = frame_count == 0 or frame_count % keyframe_interval == 0
                encoded = encode_keyframe(frame) if keyframe else encode_delta(previous, frame)
                output.write(bytes((0 if keyframe else 1,)))
                output.write(struct.pack("<I", len(encoded)))
                output.write(encoded)
                previous = frame
                frame_count += 1
            process.stdout.close()
            stderr = process.stderr.read() if process.stderr else b""
            status = process.wait()
            if status:
                raise RuntimeError(stderr.decode(errors="replace").strip() or "ffmpeg failed")
            if not frame_count:
                raise RuntimeError("video contained no decodable frames")
            output.seek(6)
            output.write(struct.pack("<H", frame_count))
            output.flush()
            output.seek(0, 2)
            output_size = output.tell()
    except Exception:
        if process.poll() is None:
            process.kill()
            process.wait()
        destination.unlink(missing_ok=True)
        raise
    print(f"Wrote {frame_count} frames at {fps} FPS ({output_size:,} bytes): {destination}")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("video", type=Path, help="input video supported by ffmpeg")
    parser.add_argument("-o", "--output", type=Path, help="output file (default: assets/<name>.pwevid)")
    parser.add_argument("--fps", type=int, default=10, help="playback rate from 1 to 30 (default: 10)")
    parser.add_argument("--seconds", type=float, help="optional maximum clip length; by default convert the full video")
    parser.add_argument("--keyframe-seconds", type=int, default=2, help="full frame interval (default: 2 seconds)")
    args = parser.parse_args()
    source = args.video.expanduser().resolve()
    if not source.is_file():
        parser.error(f"video not found: {source}")
    if not 1 <= args.fps <= 30 or (args.seconds is not None and args.seconds <= 0) or args.keyframe_seconds <= 0:
        parser.error("FPS must be 1-30 and provided durations must be positive")
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
