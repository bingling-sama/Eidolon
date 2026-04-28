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
- Match `--skin-type classic` or `--skin-type slim` to the skin's arm geometry.
- OBJ replacements must keep the object names listed in `architecture.md`.

## Resource Usage

```bash
cargo run -- render --skin-type slim --texture resources/bingling_sama.png
cargo run -- render --skin-type classic --texture resources/undefinedR2.png
cargo run -- convert resources/SSSSSteven.png converted.png
```

## Thanks

- `undefinedR2` for the double-layer Steve skin
- `SSSSSteven` for the single-layer Steve skin
- `bingling-sama` for the double-layer Alex skin
