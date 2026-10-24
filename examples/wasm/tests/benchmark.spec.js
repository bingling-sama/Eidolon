import { expect, test } from '@playwright/test';
import { fileURLToPath } from 'node:url';
import fs from 'node:fs';

const skinPath = fileURLToPath(
  new URL('../../../resources/bingling_sama.png', import.meta.url),
);
const capePath = fileURLToPath(
  new URL('../../../resources/Turtle_Cape.png', import.meta.url),
);

async function loadSkin(page) {
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');
  await page.locator('#file-input').setInputFiles(skinPath);
  await expect(page.locator('#btn-export')).toBeEnabled();
  await expect(page.locator('#result-hint')).toHaveClass(/hidden/);
  await page.waitForTimeout(100);
}

test.describe('Eidolon WASM Performance Benchmarks', () => {
  test('benchmark: renderer initialization and asset loading latency', async ({ page }) => {
    await page.goto('/');

    const skinBytes = Array.from(fs.readFileSync(skinPath));
    const capeBytes = Array.from(fs.readFileSync(capePath));

    const metrics = await page.evaluate(async ({ skinBytes, capeBytes }) => {
      const { CanvasRenderer } = await import('/pkg/eidolon.js');
      const dummyCanvas = document.createElement('canvas');
      dummyCanvas.width = 800;
      dummyCanvas.height = 600;

      // 1. Measure WebGPU initialization
      const t0 = performance.now();
      const benchRenderer = await CanvasRenderer.new(dummyCanvas);
      const initDuration = performance.now() - t0;

      // 2. Measure Skin upload & mesh pipeline setup
      const t1 = performance.now();
      benchRenderer.load_skin(new Uint8Array(skinBytes));
      const skinLoadDuration = performance.now() - t1;

      // 3. Measure Accessory upload & configuration
      const t2 = performance.now();
      benchRenderer.load_cape(new Uint8Array(capeBytes));
      const accessoryLoadDuration = performance.now() - t2;

      // 4. Measure initial frame presentation
      const t3 = performance.now();
      benchRenderer.render_frame();
      const firstFrameDuration = performance.now() - t3;

      return {
        initDuration,
        skinLoadDuration,
        accessoryLoadDuration,
        firstFrameDuration,
      };
    }, { skinBytes, capeBytes });

    console.log('\n================== INITIALIZATION BENCHMARK ==================');
    console.log(`WebGPU Context & CanvasRenderer Init: ${metrics.initDuration.toFixed(2)} ms`);
    console.log(`Skin Upload & Texture Setup:          ${metrics.skinLoadDuration.toFixed(2)} ms`);
    console.log(`Accessory Upload & Setup:             ${metrics.accessoryLoadDuration.toFixed(2)} ms`);
    console.log(`First Frame Presentation:             ${metrics.firstFrameDuration.toFixed(2)} ms`);
    console.log('==============================================================\n');

    expect(metrics.initDuration).toBeGreaterThan(0);
    expect(metrics.skinLoadDuration).toBeGreaterThan(0);
    expect(metrics.firstFrameDuration).toBeGreaterThan(0);
  });

  test('benchmark: animation playback frame times and stability', async ({ page }) => {
    await loadSkin(page);

    // Switch to walk animation to stress posture interpolation and mesh update
    await page.locator('#select-animation').selectOption('walk');

    const stats = await page.evaluate(async () => {
      const frameDeltas = [];
      const SAMPLE_COUNT = 90;

      return new Promise((resolve) => {
        let lastTime = performance.now();
        let count = 0;

        function recordFrame(now) {
          const delta = now - lastTime;
          lastTime = now;
          if (count > 0) {
            frameDeltas.push(delta);
          }
          count++;

          if (count <= SAMPLE_COUNT) {
            requestAnimationFrame(recordFrame);
          } else {
            frameDeltas.sort((a, b) => a - b);
            const total = frameDeltas.reduce((acc, v) => acc + v, 0);
            const avg = total / frameDeltas.length;
            const min = frameDeltas[0];
            const max = frameDeltas[frameDeltas.length - 1];
            const p50 = frameDeltas[Math.floor(frameDeltas.length * 0.5)];
            const p95 = frameDeltas[Math.floor(frameDeltas.length * 0.95)];
            const p99 = frameDeltas[Math.floor(frameDeltas.length * 0.99)];
            const drops = frameDeltas.filter(d => d > 33.33).length; // frames dropping below 30fps

            resolve({
              sampleCount: frameDeltas.length,
              avgMs: avg,
              effectiveFps: 1000 / avg,
              minMs: min,
              maxMs: max,
              p50Ms: p50,
              p95Ms: p95,
              p99Ms: p99,
              droppedFrames: drops,
            });
          }
        }

        requestAnimationFrame(recordFrame);
      });
    });

    console.log('\n================= ANIMATION FPS & FRAME TIMES ================');
    console.log(`Sampled Frames: ${stats.sampleCount}`);
    console.log(`Average Frame:  ${stats.avgMs.toFixed(2)} ms (~${stats.effectiveFps.toFixed(1)} FPS)`);
    console.log(`P50 (Median):   ${stats.p50Ms.toFixed(2)} ms`);
    console.log(`P95:            ${stats.p95Ms.toFixed(2)} ms`);
    console.log(`P99:            ${stats.p99Ms.toFixed(2)} ms`);
    console.log(`Min / Max:      ${stats.minMs.toFixed(2)} ms / ${stats.maxMs.toFixed(2)} ms`);
    console.log(`Jank (>33.3ms): ${stats.droppedFrames} frames`);
    console.log('==============================================================\n');

    expect(stats.sampleCount).toBeGreaterThanOrEqual(80);
    expect(stats.avgMs).toBeGreaterThan(0);
  });

  test('benchmark: capture_frame offscreen render and encode throughput', async ({ page }) => {
    await loadSkin(page);

    const resolutions = [
      { name: '800×600', w: 800, h: 600 },
      { name: '1080p', w: 1920, h: 1080 },
    ];
    const formats = ['png', 'webp', 'jpeg'];

    const benchmarkResults = await page.evaluate(async ({ resolutions, formats }) => {
      const renderer = window.__eidolon_renderer;
      if (!renderer) throw new Error('Renderer not available');

      // Warm-up run
      await renderer.capture_frame(400, 300, 'png');

      const records = [];
      const ITERATIONS = 3;

      for (const res of resolutions) {
        for (const fmt of formats) {
          const times = [];
          let byteSize = 0;

          for (let i = 0; i < ITERATIONS; i++) {
            const t0 = performance.now();
            const bytes = await renderer.capture_frame(res.w, res.h, fmt);
            const duration = performance.now() - t0;
            times.push(duration);
            byteSize = bytes.length;
          }

          times.sort((a, b) => a - b);
          const medianMs = times[Math.floor(times.length / 2)];
          const megapixels = (res.w * res.h) / 1_000_000;
          const throughputMpSec = (megapixels / (medianMs / 1000));

          records.push({
            resName: res.name,
            format: fmt.toUpperCase(),
            medianMs,
            sizeKb: byteSize / 1024,
            throughputMpSec,
          });
        }
      }

      return records;
    }, { resolutions, formats });

    console.log('\n================ CAPTURE & ENCODE BENCHMARK ==================');
    console.log('Resolution   Format   Latency (ms)   Size (KB)   Throughput (MP/s)');
    console.log('--------------------------------------------------------------');
    for (const r of benchmarkResults) {
      const res = r.resName.padEnd(12);
      const fmt = r.format.padEnd(8);
      const ms = r.medianMs.toFixed(1).padStart(12);
      const kb = r.sizeKb.toFixed(1).padStart(10);
      const tp = r.throughputMpSec.toFixed(2).padStart(17);
      console.log(`${res} ${fmt} ${ms} ${kb} ${tp}`);
    }
    console.log('==============================================================\n');

    expect(benchmarkResults.length).toBe(resolutions.length * formats.length);
    for (const r of benchmarkResults) {
      expect(r.medianMs).toBeGreaterThan(0);
      expect(r.sizeKb).toBeGreaterThan(0);
    }
  });

  test('benchmark: memory stability across repeated exports', async ({ page }) => {
    await loadSkin(page);

    const memStats = await page.evaluate(async () => {
      const renderer = window.__eidolon_renderer;
      if (!renderer) throw new Error('Renderer not available');

      const initialMem = performance.memory ? performance.memory.usedJSHeapSize : null;

      // Perform 10 consecutive capture operations across formats
      for (let i = 0; i < 10; i++) {
        await renderer.capture_frame(400, 300, i % 2 === 0 ? 'png' : 'webp');
      }

      const postMem = performance.memory ? performance.memory.usedJSHeapSize : null;

      return {
        initialMb: initialMem ? initialMem / (1024 * 1024) : null,
        postMb: postMem ? postMem / (1024 * 1024) : null,
      };
    });

    if (memStats.initialMb !== null && memStats.postMb !== null) {
      console.log('\n================== MEMORY STABILITY ==========================');
      console.log(`Initial Heap: ${memStats.initialMb.toFixed(2)} MB`);
      console.log(`After 10x Exports: ${memStats.postMb.toFixed(2)} MB`);
      console.log(`Delta: ${(memStats.postMb - memStats.initialMb).toFixed(2)} MB`);
      console.log('==============================================================\n');

      // Heap should not grow unreasonably (> 100MB leak for 10x 400x300 exports)
      expect(memStats.postMb - memStats.initialMb).toBeLessThan(100);
    }
  });
});
