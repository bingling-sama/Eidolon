# Resources

This project ships with sample models and skins under `resources/` for testing and demos.

## Included Assets

- `resources/classic.obj` Steve model (classic arms)
- `resources/slim.obj` Alex model (slim arms)
- `resources/bingling_sama.png` Double-layer Alex skin
- `resources/undefinedR2.png` Double-layer Steve skin
- `resources/SSSSSteven.png` Single-layer Steve skin

## Notes

- Single-layer skins are automatically expanded to double-layer when loaded if `width == height * 2`.
- Double-layer skins should be square, for example `64x64`.
- If you add your own skins, keep them as PNG files.
- Use `--slim` for Alex-style (3px arm) skins; omit it for classic (4px arm) skins.
- OBJ replacements must keep the object names listed in `architecture.md`.

## Resource Usage

```bash
cargo run -- render resources/bingling_sama.png --slim
cargo run -- render resources/undefinedR2.png
cargo run -- convert resources/SSSSSteven.png converted.png
```

## Thanks

- `undefinedR2` for the double-layer Steve skin
- `SSSSSteven` for the single-layer Steve skin
- `bingling-sama` for the double-layer Alex skin
