#!/usr/bin/env python3
"""gen_test_ir.py — generate synthetic 24-bit test IR WAVs for capture session 2.

Signature: first 5 samples carry the int24 values 0x47/0x50/0x31/0x30/0x30
("GP100") so we can grep for the IR payload on the wire/files after import.
"""
import wave, struct, random, sys

SR = 44100
MS = 250
N = SR * MS // 1000
SIG = [0x47, 0x50, 0x31, 0x30, 0x30]  # "GP100"

def samples():
    rnd = random.Random(100)
    out = []
    for i in range(N):
        if i < len(SIG):
            v = SIG[i]
        else:
            decay = (1.0 - i / N) ** 3
            v = int(rnd.uniform(-1, 1) * decay * 0x400000)
        out.append(max(-0x7FFFFF, min(0x7FFFFF, v)))
    return out

def write(path, chans):
    data = samples()
    with wave.open(path, "wb") as w:
        w.setnchannels(chans)
        w.setsampwidth(3)          # 24-bit
        w.setframerate(SR)
        frames = bytearray()
        for v in data:
            frames += struct.pack("<i", v)[0:3]          # L
            if chans == 2:
                frames += struct.pack("<i", v >> 1)[0:3] # R = L/2 (assinatura distinguível)
        w.writeframes(bytes(frames))
    print(f"{path}: {chans}ch {SR}Hz {MS}ms 24-bit, assinatura {' '.join('%02X' % s for s in SIG)}")

if __name__ == "__main__":
    base = "analysis"
    write(f"{base}/test_ir_mono.wav", 1)
    write(f"{base}/test_ir_stereo.wav", 2)
