#!/usr/bin/env python3
"""Draws rocknix/gauges.png, the Gauges card's art for the home screen.

The theme's cards are 454x1080 pixel art strips, slanted the same way, so this
draws at 1/6 size without smoothing and cuts the same shape. ES puts the name
over the middle of the strip and its help bar over the bottom 11 %, so the dials
stay clear of both.

Needs Pillow: python3 rocknix/card-art.py
"""
import math
from pathlib import Path

from PIL import Image, ImageDraw

WIDTH, HEIGHT = 454, 1080
SCALE = 6
LOW_W, LOW_H = WIDTH // SCALE + 1, HEIGHT // SCALE
# The strip's left edge at the top and bottom, and its width, from the theme's own art.
TOP_LEFT, BOTTOM_LEFT, STRIP_W = 113, 1, 340

DUSK_TOP = (24, 34, 74)
DUSK_MIDDLE = (66, 30, 84)
DUSK_BOTTOM = (150, 54, 58)
GLOW = (255, 150, 50)
BEZEL_LIGHT = (108, 116, 130)
BEZEL_DARK = (44, 48, 58)
FACE = (9, 11, 17)
TICK = (232, 226, 206)
AMBER = (255, 170, 40)
RED = (232, 44, 36)
NEEDLE = (255, 92, 36)
GREEN = (84, 214, 92)
LCD_OFF = (32, 26, 22)

# 3x5 digits.
DIGITS = {
    "0": "111101101101111", "1": "010110010010111", "2": "111001111100111",
    "3": "111001111001111", "4": "101101111001001", "5": "111100111001111",
    "6": "111100111101111", "7": "111001010010010", "8": "111101111101111",
}
BAYER = ((0, 8, 2, 10), (12, 4, 14, 6), (3, 11, 1, 9), (15, 7, 13, 5))


def strip_center(y):
    """Middle of the strip at a low-res row, in low-res columns."""
    left = TOP_LEFT + (BOTTOM_LEFT - TOP_LEFT) * (y * SCALE) / HEIGHT
    return (left + STRIP_W / 2) / SCALE


def mix(a, b, k):
    return tuple(round(p + (q - p) * k) for p, q in zip(a, b))


def dither(x, y, amount):
    """An offset for a blend amount, so colors band like the other cards' art."""
    return (BAYER[y % 4][x % 4] - 7.5) / 16 * amount


def quantize(k, steps=7):
    return round(min(1, max(0, k)) * steps) / steps


def background(px):
    for y in range(LOW_H):
        t = y / (LOW_H - 1)
        for x in range(LOW_W):
            k = quantize(t + dither(x, y, 0.14))
            if k < 0.5:
                px[x, y] = mix(DUSK_TOP, DUSK_MIDDLE, k * 2)
            else:
                px[x, y] = mix(DUSK_MIDDLE, DUSK_BOTTOM, k * 2 - 1)


def backlight(px, cx, cy, radius, reach, strength):
    """Instrument light spilling onto the panel around a dial."""
    for y in range(LOW_H):
        for x in range(LOW_W):
            d = math.hypot(x - cx, y - cy) - radius
            if 0 <= d < reach:
                k = quantize((1 - d / reach) ** 2 * strength + dither(x, y, 0.1), 6)
                px[x, y] = mix(px[x, y], GLOW, k)


def polar(cx, cy, radius, degrees):
    a = math.radians(degrees)
    return cx + radius * math.cos(a), cy + radius * math.sin(a)


def disc(draw, cx, cy, radius, color):
    draw.ellipse((cx - radius, cy - radius, cx + radius, cy + radius), fill=color)


def digit(px, x, y, char, color):
    for i, bit in enumerate(DIGITS[char]):
        if bit == "1":
            px[x + i % 3, y + i // 3] = color


def dial(img, cx, cy, radius, labels, redline, value):
    """A round gauge. labels is how many numbered steps it has; redline and value are 0..1."""
    draw = ImageDraw.Draw(img)
    px = img.load()
    start, sweep = 135, 270

    disc(draw, cx, cy, radius, BEZEL_DARK)
    # A lit upper rim, so the bezel reads as a raised ring.
    draw.arc((cx - radius, cy - radius, cx + radius, cy + radius), 190, 340, fill=BEZEL_LIGHT)
    disc(draw, cx, cy, radius - 2, FACE)

    steps = labels * 2
    for i in range(steps + 1):
        t = i / steps
        angle = start + sweep * t
        color = RED if t >= redline else TICK
        length = 4 if i % 2 == 0 else 2
        draw.line(
            (polar(cx, cy, radius - 3, angle), polar(cx, cy, radius - 3 - length, angle)),
            fill=color,
        )
    if radius >= 24:
        for i in range(labels + 1):
            x, y = polar(cx, cy, radius - 11, start + sweep * i / labels)
            digit(px, round(x) - 1, round(y) - 2, str(i), TICK if i / labels < redline else RED)

    tip = polar(cx, cy, radius - 4, start + sweep * value)
    tail = polar(cx, cy, 4, start + sweep * value + 180)
    draw.line((tail, tip), fill=NEEDLE)
    # A second line on the side away from the light makes the needle two pixels thick.
    draw.line(((tail[0] + 1, tail[1]), (tip[0] + 1, tip[1])), fill=NEEDLE)
    disc(draw, cx, cy, 3, BEZEL_LIGHT)
    disc(draw, cx, cy, 1, FACE)


def bar_graph(px, x, y, segments, lit):
    """The app's segmented bar: green, then amber, then red."""
    for i in range(segments):
        t = i / (segments - 1)
        color = GREEN if t < 0.6 else AMBER if t < 0.85 else RED
        if i >= lit:
            color = LCD_OFF
        for dy in range(5):
            # Leaning with the strip.
            lean = (4 - dy) // 3
            px[x + i * 3 + lean, y + dy] = color
            px[x + i * 3 + 1 + lean, y + dy] = color


def readout(px, x, y, whole, decimal):
    """A value like 14.7 on a dark panel, with unlit pixels showing like an LCD's."""
    width = len(whole) * 4 + 6
    for ry in range(y - 2, y + 7):
        for rx in range(x - 2, x + width + 1):
            px[rx, ry] = FACE
    for i, char in enumerate(whole + decimal):
        left = x + i * 4 + (2 if i >= len(whole) else 0)
        for row in range(5):
            for col in range(3):
                on = DIGITS[char][row * 3 + col] == "1"
                px[left + col, y + row] = AMBER if on else LCD_OFF
    px[x + len(whole) * 4, y + 4] = AMBER


def slant_mask():
    # Drawn 4x larger so the slanted edges come out smooth, like the theme's.
    k = 4
    mask = Image.new("L", (WIDTH * k, HEIGHT * k), 0)
    ImageDraw.Draw(mask).polygon(
        [
            (TOP_LEFT * k, 0),
            ((TOP_LEFT + STRIP_W) * k, 0),
            ((BOTTOM_LEFT + STRIP_W) * k, HEIGHT * k),
            (BOTTOM_LEFT * k, HEIGHT * k),
        ],
        fill=255,
    )
    return mask.resize((WIDTH, HEIGHT), Image.Resampling.LANCZOS)


def main():
    low = Image.new("RGB", (LOW_W, LOW_H))
    px = low.load()
    background(px)

    # The tachometer sits above the name, with the needle near the redline.
    y = 41
    cx = round(strip_center(y))
    backlight(px, cx, y, 27, 20, 0.7)
    dial(low, cx, y, 27, labels=8, redline=0.78, value=0.5625)

    # Under the name: a readout and a bar graph, then a small second gauge.
    y = 103
    readout(px, round(strip_center(y)) - 8, y, "14", "7")
    y = 111
    bar_graph(px, round(strip_center(y)) - 18, y, 12, 8)
    y = 133
    cx = round(strip_center(y))
    backlight(px, cx, y, 15, 12, 0.6)
    dial(low, cx, y, 15, labels=4, redline=0.8, value=0.45)

    art = low.resize((LOW_W * SCALE, LOW_H * SCALE), Image.Resampling.NEAREST)
    art = art.crop((0, 0, WIDTH, HEIGHT)).convert("RGBA")
    art.putalpha(slant_mask())
    out = Path(__file__).with_name("gauges.png")
    art.save(out, optimize=True)
    print(f"{out}: {out.stat().st_size} bytes")


if __name__ == "__main__":
    main()
