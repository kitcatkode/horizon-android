#!/usr/bin/env python3
"""Cut Material Symbols Rounded down to the icons Horizon's Material 3 interface draws.

Usage (fonttools in a virtualenv; the font and its .codepoints from google/material-design-icons,
`variablefont/MaterialSymbolsRounded[FILL,GRAD,opsz,wght].*`):

    python3 tools/material_symbols_subset.py FONT.ttf FONT.codepoints crates/horizon-theme/fonts

Writes MaterialSymbolsRounded-outlined.ttf (FILL 0) and -filled.ttf (FILL 1), both weight 400,
grade 0, optical size 24, and prints the code points for `ui_material::icon`.
"""
import sys

from fontTools import subset
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer

ICONS = """
play_arrow extension palette auto_awesome folder folder_open settings checkroom account_circle
person search download upload check check_circle close add delete arrow_back chevron_left
chevron_right expand_more expand_less refresh open_in_new content_copy logout login cloud
cloud_upload speed bug_report description tune screen_rotation monitor sports_esports error info
rotate_left rotate_right zoom_in zoom_out visibility visibility_off history more_vert keyboard
backspace send arrow_upward keyboard_capslock public storage memory aspect_ratio fullscreen link
star map inventory_2 chat landscape videocam wallpaper file_open save sync warning lock key devices
display_settings touch_app gamepad swap_horiz layers image brush grid_view radio_button_checked
radio_button_unchecked check_box check_box_outline_blank timer stop fiber_manual_record
""".split()


def main() -> None:
    src, codepoints, out_dir = sys.argv[1:4]
    table = dict(line.split() for line in open(codepoints) if line.strip())
    missing = [name for name in ICONS if name not in table]
    if missing:
        sys.exit(f"missing icons: {missing}")
    unicodes = sorted({int(table[name], 16) for name in ICONS})
    for fill, kind in [(0, "outlined"), (1, "filled")]:
        font = instancer.instantiateVariableFont(
            TTFont(src), {"FILL": fill, "GRAD": 0, "opsz": 24, "wght": 400}
        )
        options = subset.Options()
        options.layout_features = []
        options.name_IDs = ["*"]
        options.notdef_outline = True
        options.hinting = False
        subsetter = subset.Subsetter(options)
        subsetter.populate(unicodes=unicodes)
        subsetter.subset(font)
        font.save(f"{out_dir}/MaterialSymbolsRounded-{kind}.ttf")
    for name in ICONS:
        print(name, table[name])


if __name__ == "__main__":
    main()
