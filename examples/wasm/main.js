import { SkinRenderer } from './pkg/eidolon_wasm_example.js';

// ---- DOM ----
const fileInput = document.getElementById('file-input');
const btnRender = document.getElementById('btn-render');
const resultDiv = document.getElementById('result');
const statusEl = document.getElementById('status');

/** @type {SkinRenderer | null} */
let renderer = null;
/** @type {Uint8Array | null} */
let skinBytes = null;

// ---- Init ----
async function boot() {
  try {
    status('Initializing WebGPU renderer…');
    renderer = await SkinRenderer.new();
    status('Renderer ready. Select a skin PNG.');
  } catch (e) {
    status(`Init failed: ${e}`, true);
  }
}

// ---- File picker ----
fileInput.addEventListener('change', () => {
  const file = fileInput.files?.[0];
  if (!file) return;

  status(`Loading ${file.name}…`);
  const reader = new FileReader();
  reader.onload = () => {
    skinBytes = new Uint8Array(/** @type {ArrayBuffer} */ (reader.result));
    btnRender.disabled = false;
    status(`${file.name} loaded (${(skinBytes.length / 1024).toFixed(1)} KB). Click Render.`);
  };
  reader.onerror = () => status(`Failed to read file: ${reader.error}`, true);
  reader.readAsArrayBuffer(file);
});

// ---- Render ----
btnRender.addEventListener('click', async () => {
  if (!renderer || !skinBytes) return;

  btnRender.disabled = true;
  resultDiv.innerHTML = '<div class="spinner"></div>';
  status('Rendering…');

  try {
    const pngBytes = await renderer.render(skinBytes);
    const blob = new Blob([pngBytes], { type: 'image/png' });
    const url = URL.createObjectURL(blob);

    resultDiv.innerHTML = '';
    const img = document.createElement('img');
    img.src = url;
    img.alt = 'Rendered skin';
    img.onload = () => URL.revokeObjectURL(url);
    resultDiv.appendChild(img);

    status(`Done — ${(pngBytes.length / 1024).toFixed(1)} KB PNG`);
  } catch (e) {
    resultDiv.innerHTML = '<span class="placeholder">Render failed</span>';
    status(`Render error: ${e}`, true);
  } finally {
    btnRender.disabled = false;
  }
});

// ---- Helpers ----
/**
 * @param {string} msg
 * @param {boolean} [isError]
 */
function status(msg, isError) {
  statusEl.textContent = msg;
  statusEl.className = isError ? 'status error' : 'status';
}

boot();
