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
  await page.waitForTimeout(100);
}

async function renderAndShot(page) {
  return page.evaluate(async () => {
    window.__eidolon_suspend_raf = true;
    window.__eidolon_renderer.render_frame();
    await new Promise((resolve) => requestAnimationFrame(resolve));
    return document.querySelector('canvas').toDataURL();
  });
}

test('single-character visibility setters change rendered output and preserve pending state', async ({ page }) => {
  await page.emulateMedia({ reducedMotion: 'reduce' });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.goto('/');
  await expect(page.locator('#result')).toHaveAttribute('data-renderer-ready', 'true');

  await page.evaluate(() => window.__eidolon_renderer.set_outer_layers_visible(false));
  await page.locator('#file-input').setInputFiles(skinPath);
  await expect(page.locator('#btn-export')).toBeEnabled();
  const baseOnly = await renderAndShot(page);

  await page.evaluate(() => {
    window.__eidolon_renderer.set_all_parts_visible(false);
    window.__eidolon_renderer.set_part_visibility('head', true, true);
  });
  const headOnly = await renderAndShot(page);
  expect(headOnly).not.toBe(baseOnly);

  await expect(page.evaluate(() => {
    try {
      window.__eidolon_renderer.set_part_visibility('right_arm', true, true);
      return false;
    } catch {
      return true;
    }
  })).resolves.toBe(true);
});

test('visibility setters are available for scene character IDs', async ({ page }) => {
  await loadSkin(page);
  const result = await page.evaluate(async () => {
    const input = document.querySelector('#file-input');
    const bytes = new Uint8Array(await input.files[0].arrayBuffer());
    const renderer = window.__eidolon_renderer;
    const id = renderer.add_character(bytes, 'classic');
    renderer.set_character_all_parts_visible(id, false);
    renderer.set_character_part_visibility(id, 'head', true, true);
    try {
      renderer.set_character_part_visibility(id, 'torso', true, true);
      return false;
    } catch {
      return true;
    }
  });
  expect(result).toBe(true);
});
