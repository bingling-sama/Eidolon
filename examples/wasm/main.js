// Ensure GPUCanvasContext matches wasm-bindgen instanceof checks even when
// wrapped by browser extensions (e.g. devtools/installHook) or unexposed on window.
(function ensureGPUCanvasContext() {
  if (typeof window === 'undefined') return;
  if (typeof window.GPUCanvasContext === 'undefined') {
    class GPUCanvasContext {}
    Object.defineProperty(GPUCanvasContext, Symbol.hasInstance, {
      value: (inst) => inst && (inst.constructor?.name === 'GPUCanvasContext' || typeof inst.configure === 'function'),
      configurable: true,
    });
    window.GPUCanvasContext = GPUCanvasContext;
  } else {
    try {
      const Target = window.GPUCanvasContext;
      Object.defineProperty(Target, Symbol.hasInstance, {
        value: (inst) =>
          inst && (
            inst.constructor === Target ||
            inst.constructor?.name === 'GPUCanvasContext' ||
            typeof inst.configure === 'function'
          ),
        configurable: true,
      });
    } catch (_) {}
  }
})();

import { CanvasRenderer, inspect_texture } from './pkg/eidolon.js';

// ---- DOM ----
const fileInput = document.getElementById('file-input');
const fileName = document.getElementById('file-name');
const btnClearSkin = document.getElementById('btn-clear-skin');
const accessoryInput = document.getElementById('accessory-input');
const accessoryFileName = document.getElementById('accessory-file-name');
const btnUploadAccessory = document.getElementById('btn-upload-accessory');
const accessoryModeWrap = document.getElementById('accessory-mode-wrap');
const selectAccessoryMode = document.getElementById('select-accessory-mode');
const btnClearAccessory = document.getElementById('btn-clear-accessory');
const resultDiv = document.getElementById('result');
const resultHint = document.getElementById('result-hint');
const animationSelect = document.getElementById('select-animation');
const skinTypeSelect = document.getElementById('select-skin-type');
const inputElytraPitch = document.getElementById('input-elytra-pitch');
const inputElytraSpread = document.getElementById('input-elytra-spread');
const outputElytraPitch = document.getElementById('output-elytra-pitch');
const outputElytraSpread = document.getElementById('output-elytra-spread');
const btnExport = document.getElementById('btn-export');
const btnExportText = document.getElementById('btn-export-text');
const btnCopy = document.getElementById('btn-copy');
const inputSizeW = document.getElementById('input-size-w');
const inputSizeH = document.getElementById('input-size-h');
const sizePresets = document.getElementById('size-presets');
const formatPresets = document.getElementById('format-presets');
const selectExportFormat = document.getElementById('select-export-format');
const inputBgCustom = document.getElementById('input-bg-custom');
const inputBgAlpha = document.getElementById('input-bg-alpha');
const bgPresets = document.getElementById('bg-presets');
const btnPlay = document.getElementById('btn-play');
const inputTimeline = document.getElementById('input-timeline');
const outputTimeline = document.getElementById('output-timeline');
const selectSpeed = document.getElementById('select-speed');
const inputLoop = document.getElementById('input-loop');
const btnResetElytra = document.getElementById('btn-reset-elytra');
const inputNametag = document.getElementById('input-nametag');
const inputNametagSeeThrough = document.getElementById('input-nametag-see-through');

const canvas = document.createElement('canvas');
canvas.style.display = 'block';
canvas.style.maxWidth = '100%';
canvas.style.height = 'auto';
canvas.style.background = 'transparent';
resultDiv.appendChild(canvas);

/** @type {CanvasRenderer | null} */
let renderer = null;
let skinLoaded = false;
let accessoryLoaded = false;
let elytraOverridden = false;

function hasModel() {
  return skinLoaded || accessoryLoaded;
}

function updateModelActiveState() {
  const active = hasModel();
  if (active) {
    resultHint.classList.add('hidden');
    btnCopy.disabled = false;
    btnExport.disabled = false;
    inputSizeW.disabled = false;
    inputSizeH.disabled = false;
    resultDiv.style.cursor = 'grab';
  } else {
    resultHint.classList.remove('hidden');
    btnCopy.disabled = true;
    btnExport.disabled = true;
    inputSizeW.disabled = true;
    inputSizeH.disabled = true;
    resultDiv.style.cursor = 'default';
  }
}

// Camera state
const camera = { yaw: 180, pitch: 90, scale: 1.0 };
const bg = { r: 0, g: 0, b: 0, a: 0 };
const reduceMotion = window.matchMedia('(prefers-reduced-motion: reduce)').matches;
const playback = {
  playing: !reduceMotion,
  speed: 1,
  time: 0,
  duration: 4,
  loop: true,
  lastTimestamp: null,
};

// ---- Init ----
async function boot() {
  if (typeof window !== 'undefined' && !window.isSecureContext) {
    showError(
      'Insecure Context (WebGPU disabled)',
      `WebGPU is only available over HTTPS or localhost. Current origin: ${window.location.origin}. Please open http://localhost:${window.location.port || '5173'} instead of IP address.`,
    );
    return;
  }

  // Fallback to WebGL2 when navigator.gpu is unavailable.
  // wgpu automatically detects and falls back to WebGL2 on wasm32.
  try {
    console.log('Initializing WebGPU renderer…');
    renderer = await CanvasRenderer.new(canvas);
    window.__eidolon_renderer = renderer;
    resizeCanvas();
    setActivePreset(bgPresets.querySelector('.bg-swatch.transparent'));
    syncBackground();
    renderer.set_animation(animationSelect.value);
    playback.duration = renderer.animation_duration();
    renderer.set_animation_speed(playback.speed);
    renderer.set_animation_loop(playback.loop);
    renderer.set_animation_playing(playback.playing);
    playback.time = renderer.animation_time();
    updatePlaybackControls();
    setElytraAngleControls(renderer.elytra_pitch(), renderer.elytra_spread());
    if (inputNametag.value) {
      renderer.set_nametag(inputNametag.value);
    }
    renderer.set_nametag_see_through(inputNametagSeeThrough.checked);
    resultDiv.dataset.rendererReady = 'true';
    console.log('Renderer ready');
    requestAnimationFrame(frame);
  } catch (e) {
    showError('Renderer initialization failed. Ensure WebGPU or WebGL2 is supported.', String(e));
  }
}

// ---- Canvas sizing ----
function resizeCanvas() {
  const rect = resultDiv.getBoundingClientRect();
  const dpr = window.devicePixelRatio || 1;
  const w = Math.floor(rect.width * dpr);
  const h = Math.max(1, Math.floor(rect.width * 0.75 * dpr));
  if (canvas.width !== w || canvas.height !== h) {
    canvas.width = w;
    canvas.height = h;
    canvas.style.width = rect.width + 'px';
    if (renderer) renderer.resize(w, h);
  }
}
window.addEventListener('resize', resizeCanvas);

// ---- rAF render loop ----
function frame(timestamp) {
  if (renderer && !window.__eidolon_suspend_raf) {
    if (hasModel()) {
      const wasPlaying = playback.playing;
      if (playback.lastTimestamp != null) {
        const deltaSeconds = Math.max(0, (timestamp - playback.lastTimestamp) / 1000);
        try {
          renderer.advance_animation(deltaSeconds);
        } catch (e) {
          console.error('Animation advance failed:', e);
        }
      }
      playback.lastTimestamp = timestamp;
      playback.time = renderer.animation_time();
      playback.playing = renderer.animation_playing();
      if (playback.playing !== wasPlaying) updatePlayButton();
      if (!elytraOverridden) {
        setElytraAngleControls(renderer.elytra_pitch(), renderer.elytra_spread());
      }
      updateTimeline();

      renderer.set_camera(camera.yaw, camera.pitch, camera.scale);
      try {
        renderer.render_frame();
      } catch (e) {
        console.error(e);
      }
    } else {
      playback.lastTimestamp = null;
    }
  }
  requestAnimationFrame(frame);
}

document.addEventListener('visibilitychange', () => {
  playback.lastTimestamp = null;
});

// ---- Controls ----
animationSelect.addEventListener('change', () => {
  if (!renderer) return;
  renderer.set_animation(animationSelect.value);
  playback.duration = renderer.animation_duration();
  playback.time = renderer.animation_time();
  playback.lastTimestamp = null;
  setElytraAngleControls(renderer.elytra_pitch(), renderer.elytra_spread());
  updatePlaybackControls();
});

btnPlay.addEventListener('click', () => {
  if (!renderer) return;
  if (renderer.animation_finished()) renderer.sample_animation(0);
  renderer.set_animation_playing(!renderer.animation_playing());
  playback.playing = renderer.animation_playing();
  playback.lastTimestamp = null;
  updatePlayButton();
});

inputTimeline.addEventListener('input', () => {
  const time = Number(inputTimeline.value);
  playback.lastTimestamp = null;
  if (renderer) {
    renderer.sample_animation(time);
    playback.time = renderer.animation_time();
    playback.playing = renderer.animation_playing();
  }
  updatePlayButton();
  updateTimeline();
});

selectSpeed.addEventListener('change', () => {
  playback.speed = Number(selectSpeed.value);
  if (renderer) renderer.set_animation_speed(playback.speed);
  playback.lastTimestamp = null;
});

inputLoop.addEventListener('change', () => {
  playback.loop = inputLoop.checked;
  if (renderer) renderer.set_animation_loop(playback.loop);
});

function updatePlaybackControls() {
  inputTimeline.max = playback.duration;
  selectSpeed.value = String(playback.speed);
  inputLoop.checked = playback.loop;
  updatePlayButton();
  updateTimeline();
}

function updatePlayButton() {
  btnPlay.innerHTML = playback.playing ? '&#10074;&#10074;' : '&#9654;';
  btnPlay.setAttribute('aria-label', playback.playing ? 'Pause Animation' : 'Play Animation');
  btnPlay.title = playback.playing ? 'Pause Animation' : 'Play Animation';
}

function updateTimeline() {
  inputTimeline.value = playback.time;
  outputTimeline.value = `${playback.time.toFixed(2)} / ${playback.duration.toFixed(2)}`;
}

function setElytraAngleControls(pitch, spread) {
  inputElytraPitch.value = pitch;
  inputElytraSpread.value = spread;
  updateElytraAngleOutputs();
}

function updateElytraAngleOutputs() {
  const pitch = Number(inputElytraPitch.value);
  const spread = Number(inputElytraSpread.value);
  outputElytraPitch.value = `${pitch}°`;
  outputElytraSpread.value = `${spread}°`;
}

function syncElytraAngles() {
  elytraOverridden = true;
  updateElytraAngleOutputs();
  const pitch = Number(inputElytraPitch.value);
  const spread = Number(inputElytraSpread.value);
  if (renderer) renderer.set_elytra_angles(pitch, spread);
}

inputElytraPitch.addEventListener('input', syncElytraAngles);
inputElytraSpread.addEventListener('input', syncElytraAngles);

btnResetElytra.addEventListener('click', () => {
  if (!renderer) return;
  renderer.reset_elytra_angles();
  elytraOverridden = false;
  setElytraAngleControls(renderer.elytra_pitch(), renderer.elytra_spread());
});

skinTypeSelect.addEventListener('change', () => {
  if (!renderer) return;
  renderer.set_skin_type(skinTypeSelect.value);
});

inputNametag.addEventListener('input', () => {
  if (!renderer) return;
  renderer.set_nametag(inputNametag.value);
});

inputNametagSeeThrough.addEventListener('change', () => {
  if (!renderer) return;
  renderer.set_nametag_see_through(inputNametagSeeThrough.checked);
});

// ---- Resolution presets ----
sizePresets.addEventListener('click', (e) => {
  const btn = /** @type {HTMLElement} */ (e.target).closest('.size-preset');
  if (!btn) return;
  const w = parseInt(btn.dataset.w, 10);
  const h = parseInt(btn.dataset.h, 10);
  if (!w || !h) return;
  inputSizeW.value = w;
  inputSizeH.value = h;
  sizePresets.querySelectorAll('.size-preset').forEach(b => b.classList.remove('active'));
  btn.classList.add('active');
});
inputSizeW.addEventListener('input', () => sizePresets.querySelectorAll('.size-preset').forEach(b => b.classList.remove('active')));
inputSizeH.addEventListener('input', () => sizePresets.querySelectorAll('.size-preset').forEach(b => b.classList.remove('active')));

// ---- Export format ----
let currentFormat = 'png';
const formatLabels = {
  png: 'PNG',
  webp: 'WebP',
  jpeg: 'JPEG',
};

function setExportFormat(format) {
  format = (format || 'png').toLowerCase();
  if (!['png', 'webp', 'jpeg'].includes(format)) return;
  currentFormat = format;

  if (formatPresets) {
    formatPresets.querySelectorAll('.format-preset').forEach(btn => {
      btn.classList.toggle('active', btn.dataset.format === currentFormat);
    });
  }
  if (selectExportFormat && selectExportFormat.value !== currentFormat) {
    selectExportFormat.value = currentFormat;
  }

  const label = formatLabels[currentFormat] || currentFormat.toUpperCase();
  btnExport.title = `Save rendered ${label}`;
  if (btnExportText) {
    btnExportText.textContent = `Export ${label}`;
  } else {
    btnExport.textContent = `Export ${label}`;
  }
}

if (formatPresets) {
  formatPresets.addEventListener('click', (e) => {
    const btn = /** @type {HTMLElement} */ (e.target).closest('.format-preset');
    if (!btn || !btn.dataset.format) return;
    setExportFormat(btn.dataset.format);
  });
}

if (selectExportFormat) {
  selectExportFormat.addEventListener('change', () => {
    setExportFormat(selectExportFormat.value);
  });
}

inputBgCustom.addEventListener('input', () => {
  setActivePreset(null);
  syncBackground();
});
inputBgAlpha.addEventListener('input', syncBackground);

bgPresets.addEventListener('click', (e) => {
  const btn = /** @type {HTMLElement} */ (e.target).closest('.bg-swatch');
  if (!btn || btn.classList.contains('custom')) return;
  const hex = btn.dataset.color;
  const alpha = btn.dataset.alpha;
  if (hex == null) return;
  setActivePreset(btn);
  inputBgCustom.value = hex;
  inputBgAlpha.value = alpha || '100';
  syncBackground();
});

function setActivePreset(el) {
  bgPresets.querySelectorAll('.bg-swatch').forEach(b => b.classList.remove('active'));
  inputBgCustom.classList.remove('active');
  if (el) el.classList.add('active');
}

function syncBackground() {
  if (!renderer) return;
  const hex = inputBgCustom.value;
  const r = parseInt(hex.slice(1, 3), 16) / 255;
  const g = parseInt(hex.slice(3, 5), 16) / 255;
  const b = parseInt(hex.slice(5, 7), 16) / 255;
  const a = parseInt(inputBgAlpha.value, 10) / 100;
  bg.r = r; bg.g = g; bg.b = b; bg.a = a;
  // Canvas may be opaque — composite against page background for preview.
  if (a < 0.01) {
    renderer.set_background(0.98, 0.98, 0.98, 1.0); // #fafafa
  } else if (a < 1.0) {
    const fa = [0.98, 0.98, 0.98]; // page bg
    renderer.set_background(
      r * a + fa[0] * (1 - a),
      g * a + fa[1] * (1 - a),
      b * a + fa[2] * (1 - a),
      1.0,
    );
  } else {
    renderer.set_background(r, g, b, 1.0);
  }
}

// ---- File picker ----
fileInput.addEventListener('change', () => {
  const file = fileInput.files?.[0];
  if (!file) return;

  if (!renderer) {
    showError(
      'Renderer not ready',
      'Renderer is not initialized. Please ensure your browser supports WebGPU or WebGL2.',
    );
    return;
  }

  fileName.textContent = file.name;
  console.log('loading:', file.name);

  const reader = new FileReader();
  reader.onload = () => {
    if (!renderer) return;
    const bytes = new Uint8Array(/** @type {ArrayBuffer} */ (reader.result));

    let detectedKind = 'skin_double_layer';
    try {
      const rep = inspect_texture(bytes);
      detectedKind = rep.kind;
      console.log('Skin inspection report:', {
        kind: rep.kind,
        dimensions: `${rep.width}x${rep.height}`,
        is_hd: rep.is_hd,
        can_auto_upgrade: rep.can_auto_upgrade,
        desc: rep.description,
      });

      // Mis-upload redirect: if user uploaded a cape/elytra into Skin picker,
      // route it directly to accessory!
      if (rep.kind === 'cape' || rep.kind === 'elytra') {
        fileInput.value = '';
        fileName.textContent = '';
        accessoryFileName.innerHTML = `<span>${file.name}</span> <span class="file-tag badge-cape">${rep.kind === 'elytra' ? 'Elytra' : 'Cape'}</span>`;
        accessoryBytes = bytes;
        selectAccessoryMode.value = rep.kind === 'elytra' ? 'elytra' : 'cape';
        applyAccessory();
        return;
      }

      // Format tag decoration
      const tagText = rep.can_auto_upgrade
        ? 'Legacy 64x32 (Auto 2x)'
        : (rep.is_hd ? `HD (${rep.hd_scale}x)` : 'Standard');
      const tagClass = rep.can_auto_upgrade ? 'badge-warn' : 'badge-skin';
      fileName.innerHTML = `<span>${file.name}</span> <span class="file-tag ${tagClass}">${tagText}</span>`;
    } catch (e) {
      console.debug('Texture inspection failed:', e);
      fileName.textContent = file.name;
    }

    renderer.load_skin(bytes);
    camera.yaw = 180; camera.pitch = 90; camera.scale = 1.0;
    skinLoaded = true;
    btnClearSkin?.classList.remove('hidden');
    updateModelActiveState();
  };
  reader.onerror = () => {
    fileName.textContent = '';
    showError(`Failed to read ${file.name}.`, reader.error?.message);
  };
  reader.readAsArrayBuffer(file);
});

btnClearSkin?.addEventListener('click', () => {
  if (!renderer) return;
  renderer.clear_skin();
  skinLoaded = false;
  fileInput.value = '';
  fileName.textContent = '';
  btnClearSkin.classList.add('hidden');
  updateModelActiveState();
});

let accessoryBytes = null;

function applyAccessory() {
  if (!renderer) return;
  if (!accessoryBytes) {
    renderer.clear_cape();
    renderer.clear_elytra();
    accessoryLoaded = false;
    btnUploadAccessory.classList.remove('hidden');
    accessoryModeWrap.classList.add('hidden');
    accessoryFileName.textContent = '';
    updateModelActiveState();
    return;
  }

  btnUploadAccessory.classList.add('hidden');
  accessoryModeWrap.classList.remove('hidden');

  const mode = selectAccessoryMode.value;
  if (mode === 'cape') {
    renderer.clear_elytra();
    renderer.load_cape(accessoryBytes);
  } else if (mode === 'elytra') {
    renderer.clear_cape();
    renderer.load_elytra(accessoryBytes);
  }
  accessoryLoaded = true;
  updateModelActiveState();
}

accessoryInput.addEventListener('change', () => {
  const file = accessoryInput.files?.[0];
  if (!file) return;

  accessoryFileName.textContent = file.name;
  console.log('loading accessory texture:', file.name);

  const reader = new FileReader();
  reader.onload = () => {
    try {
      accessoryBytes = new Uint8Array(/** @type {ArrayBuffer} */ (reader.result));
      // Try to auto-detect mode if inspector is available
      try {
        const report = inspect_texture(accessoryBytes);
        console.log('Accessory inspection report:', {
          kind: report.kind,
          dimensions: `${report.width}x${report.height}`,
          is_hd: report.is_hd,
          desc: report.description,
        });

        const kindLabel = report.kind === 'elytra' ? 'Elytra' : 'Cape';
        const tagText = report.is_hd ? `${kindLabel} (${report.hd_scale}x HD)` : kindLabel;
        accessoryFileName.innerHTML = `<span>${file.name}</span> <span class="file-tag badge-cape">${tagText}</span>`;

        if (report.kind === 'elytra') {
          selectAccessoryMode.value = 'elytra';
        } else if (report.kind === 'cape') {
          selectAccessoryMode.value = 'cape';
        }
      } catch (err) {
        console.debug('Texture inspection skipped:', err);
        accessoryFileName.textContent = file.name;
      }
      applyAccessory();
    } catch (e) {
      console.error('Failed to apply accessory:', e);
      clearAccessory();
    }
  };
  reader.onerror = () => {
    console.error(`Failed to read ${file.name}.`, reader.error?.message);
    clearAccessory();
  };
  reader.readAsArrayBuffer(file);
});

selectAccessoryMode.addEventListener('change', () => {
  if (accessoryBytes) {
    applyAccessory();
  }
});

function clearAccessory() {
  accessoryBytes = null;
  accessoryInput.value = '';
  accessoryFileName.textContent = '';
  accessoryLoaded = false;
  btnUploadAccessory.classList.remove('hidden');
  accessoryModeWrap.classList.add('hidden');
  if (renderer) {
    renderer.clear_cape();
    renderer.clear_elytra();
  }
  updateModelActiveState();
}

btnClearAccessory.addEventListener('click', clearAccessory);

// ---- Orbit interaction (drag to rotate, scroll to zoom) ----
let dragging = false;
let dragLast = { x: 0, y: 0 };

canvas.addEventListener('mousedown', (e) => {
  if (!hasModel()) return;
  dragging = true;
  dragLast = { x: e.clientX, y: e.clientY };
  canvas.style.cursor = 'grabbing';
});

window.addEventListener('mousemove', (e) => {
  if (!dragging) return;
  const dx = e.clientX - dragLast.x;
  const dy = e.clientY - dragLast.y;
  dragLast = { x: e.clientX, y: e.clientY };
  camera.yaw = (camera.yaw - dx * 0.5) % 360;
  if (camera.yaw < 0) camera.yaw += 360;
  camera.pitch = Math.min(180, Math.max(0, camera.pitch + dy * 0.5));
});

window.addEventListener('mouseup', () => {
  dragging = false;
  if (hasModel()) canvas.style.cursor = 'grab';
});

canvas.addEventListener('wheel', (e) => {
  if (!hasModel()) return;
  e.preventDefault();
  camera.scale = Math.min(4.0, Math.max(0.25, camera.scale - e.deltaY * 0.005));
}, { passive: false });

// ---- Copy to Clipboard ----
btnCopy.addEventListener('click', async () => {
  if (!renderer || !hasModel()) return;
  btnCopy.disabled = true;
  btnCopy.textContent = 'Copying…';
  try {
    const w = Math.max(64, Math.min(4096, parseInt(inputSizeW.value, 10) || 800));
    const h = Math.max(64, Math.min(4096, parseInt(inputSizeH.value, 10) || 600));
    renderer.set_background(bg.r, bg.g, bg.b, bg.a);
    window.__eidolon_suspend_raf = true;
    const pngBytes = await renderer.capture_frame(w, h, 'png');
    syncBackground();
    const blob = new Blob([pngBytes], { type: 'image/png' });

    if (!navigator.clipboard?.write) {
      throw new Error('Async Clipboard API not supported or context insecure');
    }

    await navigator.clipboard.write([
      new ClipboardItem({ 'image/png': blob }),
    ]);
    btnCopy.textContent = 'Copied!';
    setTimeout(() => {
      btnCopy.textContent = 'Copy to Clipboard';
    }, 2000);
  } catch (e) {
    console.error('Copy to clipboard failed:', e);
    btnCopy.textContent = 'Failed';
    setTimeout(() => {
      btnCopy.textContent = 'Copy to Clipboard';
    }, 2000);
  } finally {
    window.__eidolon_suspend_raf = false;
    btnCopy.disabled = false;
  }
});

// ---- Export ----
btnExport.addEventListener('click', async () => {
  if (!renderer || !hasModel()) return;
  btnExport.disabled = true;
  if (btnExportText) {
    btnExportText.textContent = 'Exporting…';
  } else {
    btnExport.textContent = 'Exporting…';
  }
  try {
    const w = Math.max(64, Math.min(4096, parseInt(inputSizeW.value, 10) || 800));
    const h = Math.max(64, Math.min(4096, parseInt(inputSizeH.value, 10) || 600));
    // Set true RGBA for export (supports transparency), then restore preview color.
    renderer.set_background(bg.r, bg.g, bg.b, bg.a);
    window.__eidolon_suspend_raf = true;
    const bytes = await renderer.capture_frame(w, h, currentFormat);
    syncBackground(); // restore preview-adapted color

    const mimeMap = {
      png: 'image/png',
      webp: 'image/webp',
      jpeg: 'image/jpeg',
    };
    const extMap = {
      png: 'png',
      webp: 'webp',
      jpeg: 'jpg',
    };
    const descMap = {
      png: 'PNG Image',
      webp: 'WebP Image',
      jpeg: 'JPEG Image',
    };

    const mime = mimeMap[currentFormat] || 'image/png';
    const ext = extMap[currentFormat] || 'png';
    const desc = descMap[currentFormat] || 'Image';
    const fileName = `rendered_skin.${ext}`;

    const blob = new Blob([bytes], { type: mime });
    if (window.showSaveFilePicker) {
      try {
        const handle = await window.showSaveFilePicker({
          suggestedName: fileName,
          types: [{ description: desc, accept: { [mime]: [`.${ext}`] } }],
        });
        const writable = await handle.createWritable();
        await writable.write(blob);
        await writable.close();
      } catch (e) {
        if (e.name !== 'AbortError') console.error('Export failed:', e);
      }
    } else {
      const url = URL.createObjectURL(blob);
      const a = document.createElement('a');
      a.href = url;
      a.download = fileName;
      a.click();
      URL.revokeObjectURL(url);
    }
    console.log('exported:', bytes.length, 'bytes');
  } catch (e) {
    console.error('Export failed:', e);
  } finally {
    window.__eidolon_suspend_raf = false;
    btnExport.disabled = false;
    setExportFormat(currentFormat);
  }
});

// ---- Helpers ----
function showError(title, detail) {
  resultDiv.innerHTML = `
    <div class="empty-state">
      <div class="icon">
        <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round">
          <circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/>
          <line x1="12" y1="16" x2="12.01" y2="16"/>
        </svg>
      </div>
      <h2>${title}</h2>
      <p>${detail || 'Try a different skin file.'}</p>
    </div>`;
}

updatePlayButton();
boot();
