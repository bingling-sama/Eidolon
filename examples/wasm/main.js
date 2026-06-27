import { CanvasRenderer } from './pkg/eidolon_wasm_example.js';

// ---- DOM ----
const fileInput = document.getElementById('file-input');
const fileName = document.getElementById('file-name');
const resultDiv = document.getElementById('result');
const resultHint = document.getElementById('result-hint');
const postureSelect = document.getElementById('select-posture');
const skinTypeSelect = document.getElementById('select-skin-type');
const btnExport = document.getElementById('btn-export');
const inputSizeW = document.getElementById('input-size-w');
const inputSizeH = document.getElementById('input-size-h');
const sizePresets = document.getElementById('size-presets');
const inputBgCustom = document.getElementById('input-bg-custom');
const inputBgAlpha = document.getElementById('input-bg-alpha');
const bgPresets = document.getElementById('bg-presets');

const canvas = document.createElement('canvas');
canvas.style.display = 'block';
canvas.style.maxWidth = '100%';
canvas.style.height = 'auto';
canvas.style.background = 'transparent';
resultDiv.appendChild(canvas);

/** @type {CanvasRenderer | null} */
let renderer = null;
let skinLoaded = false;

// Camera state
const camera = { yaw: 180, pitch: 90, scale: 1.0 };
const bg = { r: 0, g: 0, b: 0, a: 0 };

// ---- Init ----
async function boot() {
  try {
    console.log('Initializing WebGPU renderer…');
    renderer = await CanvasRenderer.new(canvas);
    resizeCanvas();
    setActivePreset(bgPresets.querySelector('.bg-swatch.transparent'));
    console.log('Renderer ready');
    requestAnimationFrame(frame);
  } catch (e) {
    showError('Renderer initialization failed. Check WebGPU support.', String(e));
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
function frame() {
  if (renderer && skinLoaded) {
    renderer.set_camera(camera.yaw, camera.pitch, camera.scale);
    try {
      renderer.render_frame();
    } catch (e) {
      console.error(e);
    }
  }
  requestAnimationFrame(frame);
}

// ---- Controls ----
postureSelect.addEventListener('change', () => {
  if (!renderer) return;
  renderer.set_posture(postureSelect.value);
});

skinTypeSelect.addEventListener('change', () => {
  if (!renderer) return;
  renderer.set_skin_type(skinTypeSelect.value);
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

  fileName.textContent = file.name;
  console.log('loading:', file.name);

  const reader = new FileReader();
  reader.onload = () => {
    const bytes = new Uint8Array(/** @type {ArrayBuffer} */ (reader.result));
    renderer.load_skin(bytes);
    camera.yaw = 180; camera.pitch = 90; camera.scale = 1.0;
    skinLoaded = true;
    resultHint.classList.add('hidden');
    btnExport.disabled = false;
    inputSizeW.disabled = false;
    inputSizeH.disabled = false;
    resultDiv.style.cursor = 'grab';
  };
  reader.onerror = () => {
    fileName.textContent = '';
    showError(`Failed to read ${file.name}.`, reader.error?.message);
  };
  reader.readAsArrayBuffer(file);
});

// ---- Orbit interaction (drag to rotate, scroll to zoom) ----
let dragging = false;
let dragLast = { x: 0, y: 0 };

canvas.addEventListener('mousedown', (e) => {
  if (!skinLoaded) return;
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
  if (skinLoaded) canvas.style.cursor = 'grab';
});

canvas.addEventListener('wheel', (e) => {
  if (!skinLoaded) return;
  e.preventDefault();
  camera.scale = Math.min(4.0, Math.max(0.25, camera.scale - e.deltaY * 0.005));
}, { passive: false });

// ---- Export ----
btnExport.addEventListener('click', async () => {
  if (!renderer || !skinLoaded) return;
  btnExport.disabled = true;
  btnExport.textContent = 'Exporting…';
  try {
    const w = Math.max(64, Math.min(4096, parseInt(inputSizeW.value, 10) || 800));
    const h = Math.max(64, Math.min(4096, parseInt(inputSizeH.value, 10) || 600));
    // Set true RGBA for export (supports transparency), then restore preview color.
    renderer.set_background(bg.r, bg.g, bg.b, bg.a);
    const pngBytes = await renderer.capture_frame(w, h);
    syncBackground(); // restore preview-adapted color
    const blob = new Blob([pngBytes], { type: 'image/png' });
    if (window.showSaveFilePicker) {
      try {
        const handle = await window.showSaveFilePicker({
          suggestedName: 'rendered_skin.png',
          types: [{ description: 'PNG Image', accept: { 'image/png': ['.png'] } }],
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
      a.download = 'rendered_skin.png';
      a.click();
      URL.revokeObjectURL(url);
    }
    console.log('exported:', pngBytes.length, 'bytes');
  } catch (e) {
    console.error('Export failed:', e);
  } finally {
    btnExport.disabled = false;
    btnExport.textContent = 'Export PNG';
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

boot();
