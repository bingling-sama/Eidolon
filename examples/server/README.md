# Eidolon HTTP Rendering Server

A resident high-performance HTTP microservice for offscreen Minecraft skin rendering.

## Advantages

- **Zero GPU Cold Start**: `Renderer`, WGPU context, shaders, and OBJ models remain resident in memory. Per-request latency drops from ~400ms down to ~10–25ms.
- **Concurrent Worker Pool**: Render tasks are dispatched to dedicated worker threads with queue buffering and backpressure.
- **In-Memory Image Caching**: Network-fetched skins/capes (e.g. from Mojang CDN) are cached in memory using `moka` LRU cache.
- **Zero Disk Writes**: Rendered frames are encoded directly into memory buffers and streamed to the HTTP response.

## Quick Start

```bash
# Set headless rendering environment (if running without physical display/GPU)
export LIBGL_ALWAYS_SOFTWARE=1
export MESA_LOADER_DRIVER_OVERRIDE=llvmpipe

# Run server
cargo run --release -p eidolon-server -- --port 8080 --workers 2
```

## CLI / Environment Options

| Flag | Env Variable | Default | Description |
|---|---|---|---|
| `--host` | `EIDOLON_HOST` | `0.0.0.0` | Bind address |
| `-p, --port` | `EIDOLON_PORT` | `8080` | Port to listen on |
| `-w, --workers` | `EIDOLON_WORKERS` | `2` | Number of dedicated GPU worker threads |
| `--queue-capacity` | `EIDOLON_QUEUE_CAPACITY` | `256` | Maximum pending render queue depth |
| `--cache-entries` | `EIDOLON_CACHE_ENTRIES` | `2048` | Max in-memory texture cache count |
| `--cache-ttl` | `EIDOLON_CACHE_TTL` | `3600` | Texture cache TTL in seconds |

## API Endpoints

### 1. `GET /render` / `POST /render`

Render a player skin with optional accessories and posture settings.

#### Parameters

| Parameter | Type | Default | Description |
|---|---|---|---|
| `skinUrl` (or `skin_url`, `skin`) | string | **Required** | URL or local path to player skin PNG |
| `capeUrl` (or `cape_url`, `cape`) | string | `None` | URL or local path to cape PNG |
| `elytraUrl` (or `elytra_url`, `elytra`) | string | `None` | URL or local path to elytra PNG |
| `slim` | bool / string | `false` | Alex slim arm geometry (`true`, `false`, `1`, `0`) |
| `posture` | string | `idle` | `idle`, `walk`, `run`, `wave`, `fly` |
| `width` | u32 | `512` | Image width in pixels |
| `height` | u32 | `512` | Image height in pixels |
| `format` | string | `png` | `png` or `webp` |
| `nametag` | string | `None` | Player nameplate rendered above head |
| `cam_yaw` | f32 | `180.0` | Camera orbit yaw (degrees) |
| `cam_pitch` | f32 | `90.0` | Camera orbit pitch (degrees) |
| `cam_zoom` | f32 | `1.0` | Camera zoom multiplier |

#### Example Request

```bash
curl "http://127.0.0.1:8080/render?skinUrl=https://textures.minecraft.net/texture/xxx&capeUrl=https://textures.minecraft.net/texture/yyy&slim=1&posture=wave" -o player.png
```

### 2. `GET /health`

Healthcheck endpoint. Returns JSON:

```json
{
  "status": "ok",
  "uptime_secs": 120
}
```

## Python Integration (`skinrendermcapi.py`)

Replace `subprocess.run(["eidolon", "render", ...])` with a direct HTTP request:

```python
import httpx

def render_skin(skin_url: str, cape_url: str = None, is_slim: bool = False) -> bytes:
    response = httpx.get(
        "http://127.0.0.1:8080/render",
        params={
            "skinUrl": skin_url,
            "capeUrl": cape_url,
            "slim": is_slim,
            "posture": "idle",
            "width": 512,
            "height": 512,
        },
        timeout=10.0,
    )
    response.raise_for_status()
    return response.content
```
