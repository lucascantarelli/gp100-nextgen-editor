#!/usr/bin/env python3
"""Extract offset-tagged strings from GP-100.exe and hunt protocol tokens."""
import re, sys
sys.stdout.reconfigure(errors="replace")

d = open("analysis/nsis_app/GP-100.exe", "rb").read()

out = []
for m in re.finditer(rb"[\x20-\x7e]{5,}", d):
    out.append((m.start(), m.group().decode()))
for m in re.finditer(rb"(?:[\x20-\x7e]\x00){5,}", d):
    out.append((m.start(), m.group().decode("utf-16-le")))
out.sort()

with open("analysis/exe_strings.txt", "w", encoding="utf-8") as f:
    for off, s in out:
        f.write(f"0x{off:08X} {s}\n")
print(f"strings: {len(out)} -> analysis/exe_strings.txt")

pats = ["midiOut", "midiIn", "MIDIHDR", "midiStream", "SysEx", "sysex", "MIDI_IO",
        "CALLBACK_FUNCTION", "F0", "0xF0", "240,", "vid", "pid", "VID", "PID",
        "USB", "hid", "HID", "CRC", "crc", "checksum", "packet", "Packet",
        "handshake", "Handshake", "ACK", "command", "Command", "cmd_",
        "PRESET_", "preset_", "GET_", "SET_", "0x00F0", "deviceID", "DeviceID"]
seen = set()
for off, s in out:
    for p in pats:
        if p in s and s not in seen:
            seen.add(s)
            print(f"0x{off:08X} {s[:120]}")
            break
