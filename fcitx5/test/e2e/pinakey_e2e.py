#!/usr/bin/env python3
"""
E2E test cho PinaKey — chạy fcitx5 THẬT + dbusfrontend production, bơm phím qua D-Bus, kiểm chuỗi ra.

Khác với test in-process (testfrontend) trong fcitx5/test/, đây là kiểm thử đầu-cuối: PinaKey đã được
CÀI vào hệ thống (hoặc ~/.local), fcitx5 nạp addon qua đường tải thật, frontend D-Bus là frontend mà
các ứng dụng GTK/Qt dùng. Driver tạo input context, focus, gõ phím, rồi:
  - chế độ preedit: gom signal CommitString.
  - chế độ không-gạch-chân (bật cap SurroundingText): gom CommitString + DeleteSurroundingText, dựng
    lại nội dung tài liệu.
Trả mã thoát != 0 nếu có ca sai.

Chạy: được bọc bởi tools/run-e2e.sh (đặt sẵn profile + addon dir + dbus-run-session).
"""
import os
import sys
import time
import subprocess

import dbus
import dbus.mainloop.glib
from gi.repository import GLib

dbus.mainloop.glib.DBusGMainLoop(set_as_default=True)

FCITX = "org.fcitx.Fcitx5"
IM1 = "org.fcitx.Fcitx.InputMethod1"
IC1 = "org.fcitx.Fcitx.InputContext1"
CTL1 = "org.fcitx.Fcitx.Controller1"

CAP_PREEDIT = 1 << 1
CAP_SURROUNDING = 1 << 6

# Keysym đặc biệt (X11). Ký tự ASCII in được dùng thẳng mã codepoint.
KEYSYMS = {" ": 0x20, "space": 0x20, "return": 0xFF0D, "backspace": 0xFF08, "escape": 0xFF1B}


def keysym(token):
    if len(token) == 1:
        return ord(token)
    return KEYSYMS[token.lower()]


class Fcitx:
    def __init__(self):
        self.bus = dbus.SessionBus()
        env = dict(os.environ)
        addons = "keyboard,dbus,dbusfrontend,pinakey"
        self.proc = subprocess.Popen(
            ["fcitx5", "--disable=all", "--enable=" + addons],
            env=env, stdout=open("/tmp/pinakey_e2e_fcitx5.log", "w"),
            stderr=subprocess.STDOUT,
        )
        for _ in range(80):
            if self.bus.name_has_owner(FCITX):
                break
            time.sleep(0.25)
        else:
            raise RuntimeError("fcitx5 không lên bus (xem /tmp/pinakey_e2e_fcitx5.log)")
        time.sleep(0.6)
        self.im = dbus.Interface(self.bus.get_object(FCITX, "/org/freedesktop/portal/inputmethod"), IM1)
        self.ctl = dbus.Interface(self.bus.get_object(FCITX, "/controller"), CTL1)

    def new_ic(self, program, surrounding):
        path, _uuid = self.im.CreateInputContext([("program", program)])
        ic = dbus.Interface(self.bus.get_object(FCITX, path), IC1)
        doc = {"text": [], "cursor": 0}

        def on_commit(s):
            s = str(s)
            doc["text"][doc["cursor"]:doc["cursor"]] = list(s)
            doc["cursor"] += len(s)

        def on_delete(offset, nchar):
            start = max(0, doc["cursor"] + int(offset))
            end = min(len(doc["text"]), start + int(nchar))
            del doc["text"][start:end]
            doc["cursor"] = start

        self.bus.add_signal_receiver(on_commit, "CommitString", IC1, path=path)
        self.bus.add_signal_receiver(on_delete, "DeleteSurroundingText", IC1, path=path)
        cap = CAP_PREEDIT | (CAP_SURROUNDING if surrounding else 0)
        ic.SetCapability(dbus.UInt64(cap))
        ic.FocusIn()
        # IC mới mặc định ở keyboard-us (inactive) → chọn PinaKey cho IC đang focus.
        pump(60)
        self.ctl.SetCurrentIM("pinakey")
        pump(60)
        return ic, doc

    def stop(self):
        self.proc.terminate()
        try:
            self.proc.wait(timeout=5)
        except Exception:
            self.proc.kill()


def pump(ms=140):
    ctx = GLib.MainContext.default()
    end = time.time() + ms / 1000.0
    while time.time() < end:
        while ctx.pending():
            ctx.iteration(False)
        time.sleep(0.004)


def send(ic, sym, state=0):
    ic.ProcessKeyEvent(dbus.UInt32(sym), dbus.UInt32(0), dbus.UInt32(state),
                       dbus.Boolean(False), dbus.UInt32(0))
    pump()


def type_tokens(ic, tokens):
    for t in tokens:
        send(ic, keysym(t))


# Mỗi ca: (nhãn, surrounding?, danh sách token phím, chuỗi tài liệu mong đợi)
# Token là ký tự đơn, hoặc tên phím đặc biệt ("space"/"return"…).
CASES = [
    # --- Telex, chế độ preedit (app không có surrounding text) ---
    ("telex-1-tu", False, list("vieetj") + ["space"], "việt "),
    ("telex-tieng", False, list("tieengs") + ["space"], "tiếng "),
    ("telex-fallback", False, list("loz") + ["space"], "loz "),
    ("telex-dau-sac", False, list("as") + ["space"], "á "),
    # --- Gõ không gạch chân (app có SurroundingText) ---
    ("nounderline-cau", True, list("tieengs vieetj"), "tiếng việt"),
    ("nounderline-daumu", True, list("ddaay laf tieengs vieetj"), "đây là tiếng việt"),
    ("nounderline-1tu", True, list("dduongwf"), "đường"),
    # --- Emoji theo mã hex ---
    ("emoji-hex", False, [":"] + list("u1f600") + ["return"], "\U0001F600"),
]


def main():
    fx = Fcitx()
    failures = []
    try:
        for label, surrounding, tokens, expected in CASES:
            ic, doc = fx.new_ic("e2e-" + label, surrounding)
            type_tokens(ic, tokens)
            pump(250)
            got = "".join(doc["text"])
            ok = got == expected
            print(f"[{'PASS' if ok else 'FAIL'}] {label}: -> {got!r} (mong {expected!r})")
            if not ok:
                failures.append(label)
            try:
                ic.FocusOut()
                ic.DestroyIC()
            except Exception:
                pass
    finally:
        fx.stop()

    print(f"=== KẾT QUẢ E2E: {len(CASES) - len(failures)}/{len(CASES)} pass ===")
    sys.exit(1 if failures else 0)


if __name__ == "__main__":
    main()
