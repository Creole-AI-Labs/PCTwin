"""Makes the tray and menu-bar icons from the approved symbol (design/PCTwin_Logo_Symbol.png in the
private repository), without redrawing it: the symbol's own shape becomes the transparency, filled
with one colour.

    python scripts/make-tray-icons.py <PCTwin_Logo_Symbol.png>

Writes app/src-tauri/icons/tray/tray-black.png (macOS template image; light Windows taskbars) and
tray-white.png (dark Windows taskbars), each 64 x 64 with a small margin, and the same pixels as
raw RGBA (.rgba), which the app loads directly so it needs no image decoder.
"""
import sys
from pathlib import Path

from PIL import Image

SIZE = 64
MARGIN = 3

# Flatten onto white first: a transparent background would otherwise read as black (all ink).
raw = Image.open(sys.argv[1]).convert("RGBA")
src = Image.new("RGBA", raw.size, (255, 255, 255, 255))
src.alpha_composite(raw)
src = src.convert("RGB")
# Ink is how far a pixel is from the white background (the mark is blue, about RGB 25, 80, 250).
red = src.getchannel("R")
alpha = red.point(lambda r: max(0, min(255, round((255 - r) * 255 / 220))))
box = alpha.point(lambda a: 255 if a > 40 else 0).getbbox()
alpha = alpha.crop(box)
w, h = alpha.size
side = max(w, h)
square = Image.new("L", (side, side), 0)
square.paste(alpha, ((side - w) // 2, (side - h) // 2))
inner = SIZE - 2 * MARGIN
mask = square.resize((inner, inner), Image.LANCZOS)

out = Path(__file__).resolve().parent.parent / "app" / "src-tauri" / "icons" / "tray"
out.mkdir(parents=True, exist_ok=True)
for name, colour in (("tray-black.png", (0, 0, 0)), ("tray-white.png", (255, 255, 255))):
    icon = Image.new("RGBA", (SIZE, SIZE), colour + (0,))
    glyph = Image.new("RGBA", (inner, inner), colour + (255,))
    glyph.putalpha(mask)
    icon.paste(glyph, (MARGIN, MARGIN), glyph)
    icon.save(out / name, optimize=True)
    (out / name.replace(".png", ".rgba")).write_bytes(icon.tobytes())
    print(out / name)
