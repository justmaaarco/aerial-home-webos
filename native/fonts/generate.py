"""Create static Inter instances from the web launcher's exact existing font.

Run in an isolated environment containing fonttools[woff]==4.68.0.
No font download is performed. Font files remain under Inter's OFL 1.1.
"""

import hashlib
from pathlib import Path
import shutil

from fontTools.ttLib import TTFont
from fontTools.varLib.instancer import instantiateVariableFont


DIRECTORY = Path(__file__).resolve().parent
SOURCE = DIRECTORY.parents[1] / "app" / "assets" / "InterVariable.woff2"
SOURCE_SHA256 = "693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3"
STANDARD = {300: "Light", 400: "Regular", 500: "Medium", 600: "SemiBold", 700: "Bold"}
EXACT = (240, 260, 420, 450, 550, 650, 720)
DISPLAY = (240, 260, 550, 650, 720)


def instance(weight, optical_size, filename, named):
    font = TTFont(SOURCE, recalcTimestamp=False)
    instantiateVariableFont(
        font, {"opsz": optical_size, "wght": weight}, inplace=True, updateFontNames=named
    )
    font.flavor = None
    if not named:
        family = "Inter Display" if optical_size == 32 else "Inter"
        style = "Weight " + str(weight)
        names = {
            1: family,
            2: style,
            4: family + " " + style,
            6: family.replace(" ", "") + "-Weight" + str(weight),
            16: family,
            17: style,
        }
        for record in font["name"].names:
            if record.nameID in names:
                record.string = names[record.nameID].encode(record.getEncoding())
        font["name"].removeNames(nameID=25)
    destination = DIRECTORY / filename
    font.save(destination)
    check = TTFont(destination)
    assert "fvar" not in check and "gvar" not in check
    assert check["OS/2"].usWeightClass == weight
    assert destination.read_bytes()[:4] == b"\x00\x01\x00\x00"
    print(filename, destination.stat().st_size, "bytes", "weight", weight, "opsz", optical_size)


def main():
    if hashlib.sha256(SOURCE.read_bytes()).hexdigest() != SOURCE_SHA256:
        raise ValueError("Existing web Inter source changed; review before regenerating")
    for weight, style in STANDARD.items():
        instance(weight, 14, "Inter-" + style + ".ttf", True)
    instance(600, 32, "Inter-DisplaySemiBold.ttf", True)
    for weight in EXACT:
        instance(weight, 14, "Inter-" + str(weight) + ".ttf", False)
    for weight in DISPLAY:
        instance(weight, 32, "Inter-Display-" + str(weight) + ".ttf", False)
    shutil.copyfile(SOURCE.with_name("Inter-LICENSE.txt"), DIRECTORY / "Inter-LICENSE.txt")


if __name__ == "__main__":
    main()
