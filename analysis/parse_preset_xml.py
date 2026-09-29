#!/usr/bin/env python3
"""Parse the official preset.xml shipped with the app (templates/defaults)."""
import sys
import xml.etree.ElementTree as ET
sys.stdout.reconfigure(errors="replace")

root = ET.parse("analysis/preset.xml").getroot()

def walk(e, depth=0):
    attrs = " ".join(f'{k}="{v}"' for k, v in e.attrib.items())
    print("  " * depth + f"<{e.tag} {attrs}>")
    for c in e:
        walk(c, depth + 1)

walk(root)
