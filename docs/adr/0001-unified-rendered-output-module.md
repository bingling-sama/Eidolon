# Unified rendered-output module

Status: accepted

Eidolon will replace `Renderer::render_to_image` and the CLI-owned animation export path with one library-side rendered-output module. The module owns output intent, path resolution, format validation, encoder selection, animation timing, and file writing; explicit format is authoritative, extensions are normalized case-insensitively while preserving parent directories, and successful writes return the final path. Renderer supplies RGBA frames through a frame producer seam, keeping Posture and Character state outside output policy so output behavior can be tested without GPU setup. Static output supports PNG and WebP; animation supports GIF and Animated WebP; existing files are overwritten by default. Output failures use a dedicated `EidolonError` variant, while the CLI retains its own `..` path rejection policy.

## Considered options

- Keep path and format resolution split across CLI, Renderer, and animation export. Rejected because it leaves multiple policy owners and caused divergent extension handling.
- Keep `Renderer::render_to_image` as a compatibility facade. Rejected because this is an intentional breaking change and the old interface preserves the shallow split.
- Let file extensions determine format. Rejected because explicit output intent is clearer for library callers and makes format validation consistent.

## Consequences

- Documentation, examples, and tests must migrate in the same change.
- PNG/WebP/GIF encoders remain internal adapters behind the rendered-output seam.
- The public error enum gains an output-specific variant.
