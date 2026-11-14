import { expect, test, type Page } from '@playwright/test';

/** Check the rendered words, including wrapped helper lines, inside the capsule. */
async function escapedDockText(page: Page): Promise<string[]> {
  return page.locator('.dock').evaluate((dock) => {
    const box = dock.getBoundingClientRect(), faults: string[] = [];
    const scene = dock.querySelector('.scene.on');
    if (scene === null) return ['no active dock'];
    for (const part of scene.querySelectorAll('.say h2, .say > p, button')) {
      const range = document.createRange(); range.selectNodeContents(part);
      for (const line of range.getClientRects()) {
        if (line.left < box.left || line.right > box.right || line.top < box.top || line.bottom > box.bottom) faults.push(part.textContent);
      }
    }
    return faults;
  });
}

test('the made-up case reaches Place and returns to its 3D spine', async ({ page }, info) => {
  test.setTimeout(180_000);
  await page.setViewportSize({ width: 1440, height: 900 });
  await page.goto('/');
  await expect(page.getByTestId('engine-status')).toHaveAttribute('data-status', 'ready');
  const dock = page.locator('.dock .scene.on');
  const steps = page.getByRole('navigation', { name: 'Steps', exact: true });
  await dock.getByRole('button', { name: 'Use the sample case', exact: true }).click();
  await expect(dock.getByRole('heading', {level:2})).toHaveText('All four films are in.', { timeout: 90_000 });
  await dock.getByRole('button', { name: 'Find the spine', exact: true }).click();
  await expect(dock.getByRole('heading', {level:2})).toHaveText(/^\d+ vertebrae found/, { timeout: 90_000 });
  await dock.getByRole('button', { name: 'Build the 3D spine', exact: true }).click();
  await expect(steps.getByRole('button', { name: 'Place', exact: true })).toBeDisabled();
  await expect(dock.getByRole('button', { name: 'Place screws and rods', exact: true })).toBeVisible({ timeout: 60_000 });
  const canvas = page.locator('.spine-3d-canvas');
  await expect(canvas).toBeVisible();
  const original = await canvas.elementHandle();
  await dock.getByRole('button', { name: 'Place screws and rods', exact: true }).click();
  await expect(page.locator('.place-caption')).not.toBeVisible();
  await expect(dock.getByRole('heading', {level:2})).toHaveText(/^A first placement:/, { timeout: 60_000 });
  await expect(page.locator('html')).toHaveAttribute('data-moment', 'place');
  expect(await original.evaluate((node) => node.isConnected)).toBe(true);
  const caption = page.locator('.place-caption');
  for (const [width, height] of [[1440, 900], [900, 800], [640, 800]] as const) {
    await page.setViewportSize({ width, height });
    await expect(canvas).toBeVisible();
    const box = await canvas.boundingBox();
    expect(box?.width).toBeGreaterThan(0); expect(box?.height).toBeGreaterThan(0);
    await expect(page.locator('main.stage > .display')).not.toBeVisible();
    await expect(caption).toHaveText("Sized from published tables, not from this spine's bone.");
    await expect(caption).toBeVisible();
    await expect.poll(() => escapedDockText(page)).toEqual([]);
    const cap = await caption.boundingBox(), capsule = await page.locator('.dock').boundingBox();
    if (cap === null || capsule === null) throw new Error('the caption or dock has no box');
    expect(cap.y + cap.height).toBeLessThan(capsule.y);
    await page.screenshot({ path: info.outputPath(`place-${String(width)}.png`) });
  }
  await page.setViewportSize({ width: 1440, height: 900 });
  const viewButtons = dock.getByRole('navigation', { name: 'View', exact: true });
  const canvasBox = await canvas.boundingBox();
  if (canvasBox === null) throw new Error('the canvas has no box');
  await page.mouse.move(canvasBox.x + canvasBox.width / 2, canvasBox.y + canvasBox.height / 2);
  await page.mouse.down();
  await page.mouse.move(canvasBox.x + canvasBox.width / 2 + 60, canvasBox.y + canvasBox.height / 2, { steps: 8 });
  await page.mouse.up();
  await expect(viewButtons.locator('[aria-current="true"]')).toHaveCount(0);
  await dock.getByRole('button', { name: 'Back', exact: true }).click();
  await expect(dock.getByRole('button', { name: 'Back', exact: true })).toHaveAttribute('aria-current', 'true');
  await dock.getByRole('button', { name: /^Review the \d+$/ }).click();
  await expect(dock.locator('.place-review .num')).toHaveText(/^1 of \d+$/);
  await expect(dock.getByRole('heading', {level:2})).toHaveText('What needs a look');
  await dock.getByRole('button', { name: 'Done', exact: true }).click();
  await dock.getByRole('button', { name: 'More', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'What needs a look', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  await steps.getByRole('button', { name: '3D', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-moment', 'three');
  await expect(canvas).toBeVisible();
  expect(await original.evaluate((node) => node.isConnected)).toBe(true);
  await expect(page.locator('#measurements')).toBeVisible();
  await expect(page.locator('.place-drawing')).toHaveCount(0);
  await expect(caption).toHaveCount(0);
});
