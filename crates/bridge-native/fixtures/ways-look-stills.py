"""Draws the three stills of the ways-look mock scenario: one page laid out three ways, as flat blocks.

Run: python3 -I crates/bridge-native/fixtures/ways-look-stills.py
It writes ways-look-stacked.png, ways-look-sidebar.png and ways-look-split.png beside itself. Standard library only,
and nothing is fetched. The boxes the scenario's parts name are these blocks, in these pixels.
"""
import os
import struct
import zlib

HERE = os.path.dirname(os.path.abspath(__file__))

PAGE = (238, 241, 245)
INK = (43, 52, 66)
BLUE = (59, 110, 165)
SLATE = (40, 50, 63)
WHITE = (255, 255, 255)
LINE = (207, 214, 223)
BAR = (176, 186, 199)
TEAL = (46, 160, 154)
AMBER = (229, 160, 58)


class Canvas:
    def __init__(self, w, h, fill):
        self.w, self.h = w, h
        self.rows = [bytearray(bytes(fill) * w) for _ in range(h)]

    def rect(self, x, y, w, h, fill):
        x0, y0 = max(0, x), max(0, y)
        x1, y1 = min(self.w, x + w), min(self.h, y + h)
        if x1 <= x0:
            return
        span = bytes(fill) * (x1 - x0)
        for row in range(y0, y1):
            self.rows[row][x0 * 3:x1 * 3] = span

    def card(self, x, y, w, h):
        self.rect(x, y, w, h, LINE)
        self.rect(x + 2, y + 2, w - 4, h - 4, WHITE)

    def lines(self, x, y, w, count, gap=34, thick=14):
        for at in range(count):
            self.rect(x, y + at * gap, w - (at % 3) * 60, thick, BAR)

    def png(self, name):
        raw = b"".join(b"\x00" + bytes(row) for row in self.rows)

        def chunk(kind, data):
            body = kind + data
            return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

        out = b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", self.w, self.h, 8, 2, 0, 0, 0))
        out += chunk(b"IDAT", zlib.compress(raw, 9)) + chunk(b"IEND", b"")
        with open(os.path.join(HERE, name), "wb") as file:
            file.write(out)


def stacked():
    c = Canvas(1200, 800, PAGE)
    c.rect(0, 0, 1200, 110, BLUE)
    c.rect(48, 38, 180, 34, WHITE)
    c.rect(880, 42, 90, 26, (150, 186, 224))
    c.rect(990, 42, 90, 26, (150, 186, 224))
    for at in range(3):
        x = 60 + at * 370
        c.card(x, 150, 340, 430)
        c.rect(x + 24, 174, 292, 150, (205, 222, 240))
        c.rect(x + 24, 346, 200, 20, INK)
        c.lines(x + 24, 390, 292, 4)
    c.rect(0, 700, 1200, 100, LINE)
    c.rect(60, 736, 240, 18, BAR)
    c.png("ways-look-stacked.png")


def sidebar():
    c = Canvas(1200, 800, PAGE)
    c.rect(0, 0, 280, 800, SLATE)
    c.rect(32, 40, 150, 30, (150, 168, 190))
    for at in range(6):
        c.rect(32, 120 + at * 64, 200, 22, (92, 108, 128) if at else AMBER)
    c.rect(280, 0, 920, 100, WHITE)
    c.rect(280, 98, 920, 2, LINE)
    c.rect(320, 36, 260, 28, INK)
    c.rect(1060, 34, 100, 32, TEAL)
    c.card(320, 140, 840, 600)
    c.rect(352, 172, 360, 26, INK)
    c.lines(352, 226, 776, 6, 40, 16)
    c.rect(352, 500, 380, 200, (205, 222, 240))
    c.rect(760, 500, 368, 200, (222, 236, 226))
    c.png("ways-look-sidebar.png")


def split():
    c = Canvas(1200, 750, PAGE)
    c.rect(0, 0, 1200, 90, SLATE)
    c.rect(40, 30, 200, 30, WHITE)
    c.rect(0, 90, 520, 660, WHITE)
    c.rect(520, 90, 2, 660, LINE)
    for at in range(7):
        y = 114 + at * 88
        c.rect(24, y, 472, 72, (222, 236, 250) if at == 1 else PAGE)
        c.rect(44, y + 16, 260, 16, INK)
        c.rect(44, y + 42, 360, 12, BAR)
    c.rect(522, 90, 678, 660, (246, 248, 251))
    c.rect(570, 130, 420, 30, INK)
    c.lines(570, 190, 580, 7, 40, 16)
    c.rect(570, 520, 200, 44, BLUE)
    c.png("ways-look-split.png")


if __name__ == "__main__":
    stacked()
    sidebar()
    split()
