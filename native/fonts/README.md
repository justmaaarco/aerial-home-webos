These static TrueType faces are generated from the existing web launcher's
`app/assets/InterVariable.woff2`, with every original glyph and character mapping
retained. They use the SIL Open Font License in `Inter-LICENSE.txt`.

Source SHA-256:
`693b77d4f32ee9b8bfc995589b5fad5e99adf2832738661f5402f9978429a8e3`.

The source axes are `opsz` 14–32 (default 14) and `wght` 100–900 (default 400).
The named Light/Regular/Medium/SemiBold/Bold files use weights 300/400/500/600/700
at optical size 14. Numbered files use their exact requested CSS weights at
optical size 14. `Display` files use optical size 32 for larger text. No synthetic
bold or outline changes are applied.

To reproduce the files, install `fonttools[woff]==4.68.0` in an isolated Python
environment and run `python native/fonts/generate.py` from the repository root.
The original conversion used Brotli 1.2.1. The generator reads only the existing
local font and checks its hash before writing these static instances.
