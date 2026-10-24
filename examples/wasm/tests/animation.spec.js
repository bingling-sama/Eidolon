import { expect, test } from '@playwright/test';
import { fileURLToPath } from 'node:url';

const skinPath = fileURLToPath(
  new URL('../../../resources/bingling_sama.png', import.meta.url),
);

async function loadSkin(page) {
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');
  await page.locator('#file-input').setInputFiles(skinPath);
  await expect(page.locator('#btn-export')).toBeEnabled();
  await expect(page.locator('#result-hint')).toHaveClass(/hidden/);
  await page.waitForTimeout(250);
}

async function canvasShot(page) {
  return page.locator('canvas').screenshot({ animations: 'disabled' });
}

async function pngChannelRange(page, png) {
  return page.evaluate(async (dataUrl) => {
    const image = new Image();
    image.src = dataUrl;
    await image.decode();
    const canvas = document.createElement('canvas');
    canvas.width = image.width;
    canvas.height = image.height;
    const context = canvas.getContext('2d');
    context.drawImage(image, 0, 0);
    const pixels = context.getImageData(0, 0, canvas.width, canvas.height).data;
    let min = 255;
    let max = 0;
    for (let index = 0; index < pixels.length; index += 4) {
      min = Math.min(min, pixels[index], pixels[index + 1], pixels[index + 2]);
      max = Math.max(max, pixels[index], pixels[index + 1], pixels[index + 2]);
    }
    return max - min;
  }, `data:image/png;base64,${png.toString('base64')}`);
}

test('desktop playback updates the canvas and elytra reset restores animation control', async ({ page }) => {
  await page.setViewportSize({ width: 1280, height: 900 });
  await loadSkin(page);

  await expect(page.locator('#btn-play')).toHaveAttribute('title', 'Pause Animation');
  const firstFrame = await canvasShot(page);
  expect(await pngChannelRange(page, firstFrame)).toBeGreaterThan(20);
  await page.waitForTimeout(350);
  const laterFrame = await canvasShot(page);
  expect(laterFrame.equals(firstFrame)).toBe(false);

  await page.locator('#btn-play').click();
  await page.locator('#select-animation').selectOption('fly');
  await page.locator('#input-elytra-pitch').fill('42');
  await page.locator('#btn-reset-elytra').click();
  await page.locator('#input-timeline').fill('0.375');
  await page.locator('#input-timeline').dispatchEvent('input');

  await expect.poll(async () => Number(await page.locator('#input-elytra-pitch').inputValue()))
    .toBeCloseTo(23, 2);
  await expect.poll(async () => Number(await page.locator('#input-elytra-spread').inputValue()))
    .toBe(65);
  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
});

test('reduced motion starts paused and leaves the canvas unchanged', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 1280, height: 900 });
  await loadSkin(page);

  await expect(page.locator('#btn-play')).toHaveAttribute('title', 'Play Animation');
  const timeline = await page.locator('#input-timeline').inputValue();
  const firstFrame = await canvasShot(page);
  await page.waitForTimeout(350);
  const laterFrame = await canvasShot(page);

  expect(laterFrame.equals(firstFrame)).toBe(true);
  await expect(page.locator('#input-timeline')).toHaveValue(timeline);
});

test('mobile controls stay within the viewport', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 390, height: 844 });
  await loadSkin(page);

  expect(await page.evaluate(() => document.documentElement.scrollWidth <= innerWidth)).toBe(true);
  for (const selector of [
    '#btn-play',
    '#input-timeline',
    '#select-speed',
    '#input-loop',
    '#input-nametag',
    '#input-nametag-see-through',
    '#format-presets',
    'canvas',
  ]) {
    const box = await page.locator(selector).boundingBox();
    expect(box).not.toBeNull();
    expect(box.x).toBeGreaterThanOrEqual(0);
    expect(box.x + box.width).toBeLessThanOrEqual(390);
  }
});

test('nametag input and see-through toggle update rendered canvas', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 1280, height: 900 });
  await loadSkin(page);

  const baseFrame = await canvasShot(page);

  await page.locator('#input-nametag').fill('Steve');
  await page.waitForTimeout(100);
  const nametagFrame = await canvasShot(page);
  expect(nametagFrame.equals(baseFrame)).toBe(false);

  await page.locator('#input-nametag-see-through').uncheck();
  await page.waitForTimeout(100);
  const occludedFrame = await canvasShot(page);
  expect(occludedFrame.equals(baseFrame)).toBe(false);

  await page.locator('#input-nametag').fill('');
  await page.waitForTimeout(100);
  const clearedFrame = await canvasShot(page);
  expect(clearedFrame.equals(baseFrame)).toBe(true);
});

test('accessory only renders without skin and can switch between cape and elytra', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');

  const capePath = fileURLToPath(
    new URL('../../../resources/Turtle_Cape.png', import.meta.url),
  );

  // Upload accessory when no skin is loaded
  await page.locator('#accessory-input').setInputFiles(capePath);
  await expect(page.locator('#result-hint')).toHaveClass(/hidden/);
  await expect(page.locator('#btn-export')).toBeEnabled();
  await expect(page.locator('#btn-copy')).toBeEnabled();
  await expect(page.locator('#accessory-mode-wrap')).toBeVisible();

  await page.waitForTimeout(200);
  const capeFrame = await canvasShot(page);

  // Switch mode to elytra
  await page.locator('#select-accessory-mode').selectOption('elytra');
  await page.waitForTimeout(200);
  const elytraFrame = await canvasShot(page);

  // Cape and Elytra should have distinct visual geometry
  expect(elytraFrame.equals(capeFrame)).toBe(false);

  // Clear accessory restores empty state
  await page.locator('#btn-clear-accessory').click();
  await expect(page.locator('#result-hint')).not.toHaveClass(/hidden/);
  await expect(page.locator('#btn-export')).toBeDisabled();
});

test('skin can be uploaded and cleared with clear button', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');

  const skinPath = fileURLToPath(
    new URL('../../../resources/bingling_sama.png', import.meta.url),
  );

  await page.locator('#file-input').setInputFiles(skinPath);
  await expect(page.locator('#result-hint')).toHaveClass(/hidden/);
  await expect(page.locator('#btn-clear-skin')).toBeVisible();

  await page.locator('#btn-clear-skin').click();
  await expect(page.locator('#result-hint')).not.toHaveClass(/hidden/);
  await expect(page.locator('#btn-clear-skin')).not.toBeVisible();
  await expect(page.locator('#btn-export')).toBeDisabled();
});

test('inspect_texture correctly detects texture format in browser', async ({ page }) => {
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');

  const fs = await import('node:fs');
  const bytes = Array.from(fs.readFileSync(skinPath));

  const report = await page.evaluate(async (imgBytes) => {
    const { inspect_texture } = await import('/pkg/eidolon.js');
    const buf = new Uint8Array(imgBytes);
    const rep = inspect_texture(buf);
    return {
      kind: rep.kind,
      width: rep.width,
      height: rep.height,
      is_hd: rep.is_hd,
      can_auto_upgrade: rep.can_auto_upgrade,
    };
  }, bytes);

  expect(report.kind).toBe('skin_double_layer');
  expect(report.width).toBe(64);
  expect(report.height).toBe(64);
  expect(report.is_hd).toBe(false);
  expect(report.can_auto_upgrade).toBe(false);
});

test('export format selection updates UI and export button label', async ({ page }) => {
  await page.goto('/');
  await loadSkin(page);

  await expect(page.locator('#btn-export-text')).toHaveText('Export PNG');
  await expect(page.locator('.format-preset[data-format="png"]')).toHaveClass(/active/);

  // Switch to WebP
  await page.locator('.format-preset[data-format="webp"]').click();
  await expect(page.locator('#btn-export-text')).toHaveText('Export WebP');
  await expect(page.locator('.format-preset[data-format="webp"]')).toHaveClass(/active/);
  await expect(page.locator('.format-preset[data-format="png"]')).not.toHaveClass(/active/);

  // Switch to JPEG
  await page.locator('.format-preset[data-format="jpeg"]').click();
  await expect(page.locator('#btn-export-text')).toHaveText('Export JPEG');
  await expect(page.locator('.format-preset[data-format="jpeg"]')).toHaveClass(/active/);

  // Switch via select element
  await page.locator('#select-export-format').selectOption('png');
  await expect(page.locator('#btn-export-text')).toHaveText('Export PNG');
  await expect(page.locator('.format-preset[data-format="png"]')).toHaveClass(/active/);
});

test('renderer captures frames in png, webp, and jpeg formats with correct magic headers', async ({ page }) => {
  await page.goto('/');
  await loadSkin(page);

  const results = await page.evaluate(async () => {
    const renderer = window.__eidolon_renderer;
    if (!renderer) throw new Error('Renderer not available on window');

    const png = Array.from(await renderer.capture_frame(128, 128, 'png'));
    const webp = Array.from(await renderer.capture_frame(128, 128, 'webp'));
    const jpeg = Array.from(await renderer.capture_frame(128, 128, 'jpeg'));
    const def = Array.from(await renderer.capture_frame(128, 128));

    return { png, webp, jpeg, def };
  });

  // PNG magic: 0x89 0x50 0x4E 0x47 0x0D 0x0A 0x1A 0x0A
  expect(results.png.slice(0, 8)).toEqual([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);
  expect(results.def.slice(0, 8)).toEqual([0x89, 0x50, 0x4E, 0x47, 0x0D, 0x0A, 0x1A, 0x0A]);

  // WebP magic: 'RIFF' .... 'WEBP'
  const webpHeader = String.fromCharCode(...results.webp.slice(0, 4));
  const webpType = String.fromCharCode(...results.webp.slice(8, 12));
  expect(webpHeader).toBe('RIFF');
  expect(webpType).toBe('WEBP');

  // JPEG magic: 0xFF 0xD8 0xFF
  expect(results.jpeg.slice(0, 3)).toEqual([0xFF, 0xD8, 0xFF]);
});
