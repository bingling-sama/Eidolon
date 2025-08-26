# PyO3 usage

build and install:

```shell
cargo build
pip install maturin
maturin develop
```

then in python:

```python
import eidolon

eidolon.render_skin(
    800,                    # width
    600,                    # height
    "resources/bingling_sama.png",  # texture
    "slim",              # skin_type
    "png",                  # format
    180.0,                  # yaw
    90.0,                   # pitch
    1.0,                    # scale
    90.0,                   # head_yaw
    90.0,                   # head_pitch
    90.0,                   # left_arm_roll
    0.0,                    # left_arm_pitch
    90.0,                   # right_arm_roll
    0.0,                    # right_arm_pitch
    90.0,                   # left_leg_pitch
    90.0                    # right_leg_pitch
)
```