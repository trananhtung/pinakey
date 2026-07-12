#!/usr/bin/env python3
"""Generate `crates/pinakey-core/src/charset_def.rs` from upstream `charset_def.go`.

The legacy Vietnamese charsets are byte encodings (not UTF-8), so each Go string value is
decoded to its raw bytes and emitted as a Rust byte-string literal.

Usage:
    python3 tools/gen_charset.py [path/to/charset_def.go] [path/to/charset_def.rs]

Defaults:
    SRC  = $BAMBOO_GO_SRC/charset_def.go        (env BAMBOO_GO_SRC, else ./charset_def.go)
    OUT  = crates/pinakey-core/src/charset_def.rs (resolved relative to the repo root)

Clone the upstream Go source first, e.g.:
    git clone https://github.com/BambooEngine/bamboo-core /tmp/bamboo-core-src
    BAMBOO_GO_SRC=/tmp/bamboo-core-src python3 tools/gen_charset.py
"""
import os
import re
import sys

REPO_ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))


def default_src():
    base = os.environ.get("BAMBOO_GO_SRC", ".")
    return os.path.join(base, "charset_def.go")


def default_out():
    return os.path.join(REPO_ROOT, "crates", "pinakey-core", "src", "charset_def.rs")


SRC = sys.argv[1] if len(sys.argv) > 1 else default_src()
OUT = sys.argv[2] if len(sys.argv) > 2 else default_out()

name_re = re.compile(r'^\t"((?:\\.|[^"\\])*)":\s*\{')
entry_re = re.compile(r"^'((?:\\.|[^'\\])*)':\s*\"((?:\\.|[^\"\\])*)\",?$")
# Backtick raw-string value: content is literal text (no escape processing).
entry_raw_re = re.compile(r"^'((?:\\.|[^'\\])*)':\s*`([^`]*)`,?$")


def decode_rune(content):
    # Go rune literal content -> a single codepoint (int)
    if content.startswith("\\u"):
        return int(content[2:6], 16)
    if content.startswith("\\U"):
        return int(content[2:10], 16)
    if content.startswith("\\x"):
        return int(content[2:4], 16)
    if content == "\\\\":
        return ord("\\")
    if content == "\\'":
        return ord("'")
    if content == "\\n":
        return ord("\n")
    if content == "\\t":
        return ord("\t")
    # single unicode char
    assert len(content) == 1, f"unexpected rune literal: {content!r}"
    return ord(content)


def decode_string_to_bytes(content):
    out = bytearray()
    i = 0
    n = len(content)
    while i < n:
        c = content[i]
        if c == "\\":
            nxt = content[i + 1]
            if nxt == "u":
                cp = int(content[i + 2:i + 6], 16)
                out.extend(chr(cp).encode("utf-8"))
                i += 6
            elif nxt == "U":
                cp = int(content[i + 2:i + 10], 16)
                out.extend(chr(cp).encode("utf-8"))
                i += 10
            elif nxt == "x":
                out.append(int(content[i + 2:i + 4], 16))
                i += 4
            elif nxt == "\\":
                out.append(ord("\\")); i += 2
            elif nxt == '"':
                out.append(ord('"')); i += 2
            elif nxt == "n":
                out.append(ord("\n")); i += 2
            elif nxt == "t":
                out.append(ord("\t")); i += 2
            elif nxt == "r":
                out.append(ord("\r")); i += 2
            else:
                raise ValueError(f"unknown escape \\{nxt} in {content!r}")
        else:
            out.extend(c.encode("utf-8"))
            i += 1
    return bytes(out)


def rust_byte_str(b):
    return '"' + "".join("\\x%02x" % x for x in b) + '"'


def main():
    charsets = []  # list of (name, [(codepoint, bytes), ...])
    cur = None
    with open(SRC, encoding="utf-8") as f:
        for raw in f:
            line = raw.rstrip("\n")
            stripped = line.strip()
            m = name_re.match(line)
            if m:
                cur = (m.group(1), [])
                charsets.append(cur)
                continue
            if stripped in ("},", "}") and cur is not None:
                cur = None
                continue
            em = entry_re.match(stripped)
            if em and cur is not None:
                cp = decode_rune(em.group(1))
                bs = decode_string_to_bytes(em.group(2))
                cur[1].append((cp, bs))
                continue
            rm = entry_raw_re.match(stripped)
            if rm and cur is not None:
                cp = decode_rune(rm.group(1))
                bs = rm.group(2).encode("utf-8")  # raw string: literal text bytes
                cur[1].append((cp, bs))

    lines = []
    # Header khớp NGUYÊN VĂN file đã commit (tiếng Việt) — regen phải cho diff rỗng (#102).
    lines.append("//! Các bảng mã ký tự — được sinh ra từ `charset_def.go`.")
    lines.append("//!")
    lines.append("//! Giá trị là chuỗi byte thô (các bảng mã cũ không phải UTF-8). `\\xNN` là một byte đơn;")
    lines.append("//! các giá trị nhiều byte là byte UTF-8 (hoặc codepage) của chuỗi Go gốc.")
    lines.append("//!")
    lines.append("//! Sinh lại bằng `python3 tools/gen_charset.py` (xem CONTRIBUTING.md). Không sửa bằng tay.")
    lines.append("")
    lines.append("/// Một bảng mã: tên của nó cùng các mục `(source_char, encoded_bytes)`.")
    lines.append("pub type CharsetDef = (&'static str, Vec<(char, &'static [u8])>);")
    lines.append("")
    lines.append("/// Trả về bảng mã hoá cho mọi bảng mã không phải Unicode.")
    lines.append("pub fn charset_definitions() -> Vec<CharsetDef> {")
    lines.append("    vec![")
    for name, entries in charsets:
        safe_name = name.replace("\\", "\\\\").replace('"', '\\"')
        lines.append(f'        ("{safe_name}", vec![')
        for cp, bs in entries:
            lines.append(f"            ('\\u{{{cp:04x}}}', b{rust_byte_str(bs)}),")
        lines.append("        ]),")
    lines.append("    ]")
    lines.append("}")
    lines.append("")

    with open(OUT, "w", encoding="utf-8") as f:
        f.write("\n".join(lines))

    total = sum(len(e) for _, e in charsets)
    print(f"wrote {OUT}: charsets={len(charsets)} entries={total}")
    for name, entries in charsets:
        print(f"  {name}: {len(entries)}")


if __name__ == "__main__":
    main()
