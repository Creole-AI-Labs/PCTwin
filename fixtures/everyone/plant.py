#!/usr/bin/env python3
"""Plants the lab's things for a move of everyone, and writes the answer key.

Run as an administrator (root on Linux and macOS), from outside the product: the key is the
administrator's, kept where the mover cannot read it.

    plant.py plant <spec.tsv> <planted.json>
    plant.py key <planted.json> <mover name> <new home> <evidence.json> <key.json>

Each spec line, tab-separated: who (an account name, or `shared`), path (`~` is that person's
home), size in bytes, what must happen (`moves`, `stays` or `blind`), and how it is kept:

- `own`: the person's own, inside their home or a folder of theirs;
- `only`: a new folder outside any home that only that person (and administrators) can open;
- `shared`: a folder everyone can read;
- `locked`: a folder only administrators can open.

A planted file holds its own name and a line end, over and over, to its size (the lab's
`planted_bytes`), so a copy is checked byte for byte without keeping any of it.
"""

import json
import os
import subprocess
import sys

WINDOWS = os.name == "nt"


def planted_bytes(name, size):
    line = (name + "\n").encode("utf-8")
    whole, rest = divmod(size, len(line))
    return line * whole + line[:rest]


def ps(command):
    out = subprocess.run(
        ["powershell", "-NoProfile", "-NonInteractive", "-Command", command],
        check=True,
        capture_output=True,
        text=True,
    )
    return out.stdout.strip()


def account_id(user):
    if WINDOWS:
        return ps(f"(Get-LocalUser -Name '{user}').SID.Value")
    import pwd

    return str(pwd.getpwnam(user).pw_uid)


def home_of(user):
    if WINDOWS:
        sid = account_id(user)
        return ps(
            "(Get-CimInstance Win32_UserProfile | Where-Object { $_.SID -eq '"
            + sid
            + "' }).LocalPath"
        )
    import pwd

    return pwd.getpwnam(user).pw_dir


def icacls(*args):
    subprocess.run(["icacls", *args], check=True, capture_output=True)


def top_new_folder(path):
    """The highest folder of `path` that does not exist yet."""
    parent = os.path.dirname(path)
    top = None
    while parent and not os.path.exists(parent):
        top = parent
        parent = os.path.dirname(parent)
    return top


def keep(user, path, top, how):
    """Gives the planted file (and the folders made for it) to its person, as `how` says."""
    if WINDOWS:
        if how in ("own", "only"):
            target = top or path
            if how == "only":
                # A folder passes its rules on; a file (planted into a folder made earlier) takes
                # them plainly: on a file Windows drops rules marked to pass on, which would leave
                # it with none, readable by nobody.
                on = "(OI)(CI)" if os.path.isdir(target) else ""
                icacls(
                    target,
                    "/inheritance:r",
                    "/grant:r",
                    f"{user}:{on}F",
                    f"*S-1-5-32-544:{on}F",
                    f"*S-1-5-18:{on}F",
                )
            icacls(target, "/setowner", user, "/T", "/C", "/Q")
        elif how == "shared":
            icacls(top or path, "/grant", "*S-1-5-32-545:(OI)(CI)RX")
        elif how == "locked":
            target = top or path
            on = "(OI)(CI)" if os.path.isdir(target) else ""
            icacls(
                target,
                "/inheritance:r",
                "/grant:r",
                f"*S-1-5-32-544:{on}F",
                f"*S-1-5-18:{on}F",
            )
        return
    import pwd

    if how in ("own", "only"):
        pw = pwd.getpwnam(user)
        for p in [path] + folders_between(top, path):
            os.chown(p, pw.pw_uid, pw.pw_gid)
        if how == "only" and top:
            os.chmod(top, 0o700)
    elif how == "shared":
        for p in folders_between(top, path):
            os.chmod(p, 0o755)
        os.chmod(path, 0o644)
    elif how == "locked":
        if top:
            os.chmod(top, 0o700)


def folders_between(top, path):
    """The folders made for `path`, from `top` down."""
    if not top:
        return []
    out = []
    p = os.path.dirname(path)
    while True:
        out.append(p)
        if p == top or len(p) <= len(top):
            break
        p = os.path.dirname(p)
    return out


def plant(spec, out):
    planted = []
    for raw in open(spec, encoding="utf-8-sig"):
        line = raw.rstrip("\n")
        if not line or line.startswith("#"):
            continue
        who, path, size, expect, how = line.split("\t")
        size = int(size)
        if path.startswith("~"):
            path = home_of(who) + path[1:]
        path = os.path.normpath(path)
        top = top_new_folder(path)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        name = os.path.basename(path)
        with open(path, "wb") as f:
            f.write(planted_bytes(name, size))
        keep(who, path, top, how)
        planted.append(
            {
                "owner": "shared" if who == "shared" else account_id(who),
                "who": who,
                "path": path,
                "name": name,
                "size": size,
                "expect": expect,
            }
        )
    json.dump({"planted": planted}, open(out, "w", encoding="utf-8"), indent=1)


def key(planted, mover, new_home, evidence, out):
    planted = json.load(open(planted, encoding="utf-8-sig"))["planted"]
    evidence = json.load(open(evidence, encoding="utf-8-sig"))
    new_homes = {}
    for made in evidence.get("made", []):
        new_homes[made["account"]] = home_of(made["name"])
    json.dump(
        {
            "mover": account_id(mover),
            "new_home": new_home,
            "new_homes": new_homes,
            "planted": planted,
        },
        open(out, "w", encoding="utf-8"),
        indent=1,
    )


if __name__ == "__main__":
    if sys.argv[1:2] == ["plant"] and len(sys.argv) == 4:
        plant(sys.argv[2], sys.argv[3])
    elif sys.argv[1:2] == ["key"] and len(sys.argv) == 7:
        key(*sys.argv[2:])
    else:
        sys.exit(__doc__)
