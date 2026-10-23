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
