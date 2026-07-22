# Shared animation timeline and schedule

Status: accepted

Eidolon will keep `AnimationPreset` as the stateless Posture-sampling primitive and add two public animation abstractions. `AnimationTimeline` owns mutable playback position, speed, play/pause, seeking, loop/end behavior, and long-frame-gap policy while accepting platform-provided `Duration` deltas. `AnimationSchedule` is an immutable sequence of sample times for one offline preset cycle at a selected FPS and speed. Native preview, the wasm adapter, and rendered output will use these shared semantics instead of maintaining separate timing state machines.

## Considered options

- Keep playback state in native preview and browser JavaScript. Rejected because speed, seeking, suspension, looping, and cycle-end behavior had different owners and semantics.
- Make the timeline own `Instant` or browser timestamps. Rejected because platform clocks are adapter concerns; accepting `Duration` deltas keeps the timeline deterministic and portable.
- Replace `AnimationPreset::sample`. Rejected because it remains a useful stateless interface and preserving it avoids an unnecessary public breaking change.
- Put GIF delays and WebP timestamps in the animation module. Rejected because encoder time units and metadata belong to output adapters; only sample times belong to the animation schedule.

## Consequences

- `AnimationTimeline::new` starts paused at position zero in looping mode.
- Frame deltas over 250ms are treated as suspend/resume gaps and do not advance playback.
- One-shot playback stops on the final visible Posture rather than wrapping to the first frame.
- Input adapters continue to own platform timestamps and user-facing validation.
