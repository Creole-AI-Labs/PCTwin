"""Flips one byte of a file's data in the BACKING IMAGE of the drive it sits on.

usage: flip_byte.py <image> <file on the mounted image> <offset in the file>

Finds where the file's data really lies on the drive (filefrag extents), then XORs 0xFF into that
byte of the image file. The mounted file's cache still holds the old byte; the drive now holds the
new one. Prints the image offset flipped.
"""
import re
import subprocess
import sys

image, path, offset = sys.argv[1], sys.argv[2], int(sys.argv[3])
BLOCK = 4096
out = subprocess.run(
    ["filefrag", "-v", "-b" + str(BLOCK), path], check=True, capture_output=True, text=True
).stdout
print(out)
pattern = re.compile(r"^\s*\d+:\s*(\d+)\.\.\s*(\d+):\s*(\d+)\.\.\s*(\d+):", re.M)
target = offset // BLOCK
for m in pattern.finditer(out):
    logical_start, logical_end, phys_start = int(m[1]), int(m[2]), int(m[3])
    if logical_start <= target <= logical_end:
        image_offset = (phys_start + (target - logical_start)) * BLOCK + offset % BLOCK
        break
else:
    sys.exit("flip_byte.py: no extent covers file offset %d" % offset)
with open(image, "r+b") as f:
    f.seek(image_offset)
    old = f.read(1)
    f.seek(image_offset)
    f.write(bytes([old[0] ^ 0xFF]))
    f.flush()
print("flipped image offset %d: %#04x -> %#04x" % (image_offset, old[0], old[0] ^ 0xFF))
