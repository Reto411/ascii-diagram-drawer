#!/usr/bin/env python3
"""ASCII downscale of a PNG — lets an agent "see" a screenshot without image support.

Usage: ascii_view.py <file.png> [cols]   (default 110 cols)

Stdlib only (zlib/struct); supports non-interlaced 8-bit RGB/RGBA PNGs,
which is what gui-app's software rasterizer (gui-app/src/screenshot.rs) emits.
Brighter pixels map to denser characters: ' ' < '|' < '-' < '#' < '@'.
"""

import struct
import sys
import zlib


def load_png(path):
    with open(path, "rb") as f:
        data = f.read()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise SystemExit(f"not a PNG: {path}")
    pos = 8
    width = height = bitdepth = color = None
    idat = b""
    while pos < len(data):
        (length,) = struct.unpack(">I", data[pos : pos + 4])
        tag = data[pos + 4 : pos + 8]
        chunk = data[pos + 8 : pos + 8 + length]
        if tag == b"IHDR":
            width, height, bitdepth, color = struct.unpack(">IIBB", chunk[:10])
        elif tag == b"IDAT":
            idat += chunk
        elif tag == b"IEND":
            break
        pos += 12 + length
    if bitdepth != 8 or color not in (2, 6):
        raise SystemExit(f"unsupported PNG: bitdepth={bitdepth} color={color}")
    raw = zlib.decompress(idat)
    channels = 3 if color == 2 else 4
    stride = width * channels
    out = bytearray(height * width * channels)
    prev = bytearray(stride)
    rp = 0
    for y in range(height):
        ft = raw[rp]
        rp += 1
        line = bytearray(raw[rp : rp + stride])
        rp += stride
        if ft == 1:  # Sub
            for x in range(channels, stride):
                line[x] = (line[x] + line[x - channels]) & 0xFF
        elif ft == 2:  # Up
            for x in range(stride):
                line[x] = (line[x] + prev[x]) & 0xFF
        elif ft == 3:  # Average
            for x in range(stride):
                a = line[x - channels] if x >= channels else 0
                line[x] = (line[x] + ((a + prev[x]) >> 1)) & 0xFF
        elif ft == 4:  # Paeth
            for x in range(stride):
                a = line[x - channels] if x >= channels else 0
                b = prev[x]
                c = prev[x - channels] if x >= channels else 0
                p = a + b - c
                pa, pb, pc = abs(p - a), abs(p - b), abs(p - c)
                pr = a if (pa <= pb and pa <= pc) else (b if pb <= pc else c)
                line[x] = (line[x] + pr) & 0xFF
        prev = line
        out[y * stride : (y + 1) * stride] = line
    return width, height, channels, bytes(out)


def main():
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    path = sys.argv[1]
    cols = int(sys.argv[2]) if len(sys.argv) > 2 else 110
    w, h, ch, px = load_png(path)
    rows = max(1, cols // 2)  # terminal cells are ~2:1
    scale = "|-#@"
    for ry in range(rows):
        line = []
        y0, y1 = ry * h // rows, (ry + 1) * h // rows
        for rx in range(cols):
            x0, x1 = rx * w // cols, (rx + 1) * w // cols
            lum, n = 0, 0
            for y in range(y0, y1):
                base = (y * w) * ch
                for x in range(x0, x1):
                    i = base + x * ch
                    if ch == 4 and px[i + 3] <= 127:
                        continue
                    lum += (px[i] * 29 + px[i + 1] * 58 + px[i + 2] * 117) // 256
                    n += 1
            if n == 0:
                line.append(" ")
            else:
                v = lum // n
                line.append(scale[min(len(scale) - 1, round(v / 255 * (len(scale) - 1)))])
        print("".join(line))


if __name__ == "__main__":
    main()
