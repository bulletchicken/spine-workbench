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

/** A count's width must never choose a control's row. */
async function fixedDockRows(page: Page, control: '.views' | '.load-stops') {
  await expect.poll(() => page.locator('.dock .scene.on').evaluate((scene, selector) => {
    const words=scene.querySelector('.say')?.getBoundingClientRect(),nav=scene.querySelector(selector)?.getBoundingClientRect(),box=scene.getBoundingClientRect();
    const actions=[...scene.querySelectorAll(':scope > button')].map((button)=>({primary:button.classList.contains('primary'),box:button.getBoundingClientRect()}));
    if(words===undefined||nav===undefined||actions.length===0)return false;
    const first=actions[0];if(first===undefined)return false;
    return words.bottom<=nav.top && nav.bottom<=Math.min(...actions.map((a)=>a.box.top))
      && Math.abs(nav.left-box.left)<1 && Math.abs(nav.right-box.right)<1
      && actions.every((a)=>Math.abs(a.box.y+a.box.height/2-first.box.y-first.box.height/2)<1)
      && actions.filter((a)=>a.primary).every((a)=>Math.abs(a.box.right-box.right)<1)
      && Math.abs(first.box.left-box.left)<1;
  }, control)).toBe(true);
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
  const fittedMeasurements=await page.locator('#measurements').innerText();
  await page.screenshot({path:info.outputPath('three-before-placement.png')});
  await dock.getByRole('button', { name: 'Place screws and rods', exact: true }).click();
  await expect(page.locator('.place-caption')).not.toBeVisible();
  await expect(dock.getByRole('heading', {level:2})).toHaveText(/^A first placement:/, { timeout: 60_000 });
  await expect(page.locator('html')).toHaveAttribute('data-moment', 'place');
  expect(await original.evaluate((node) => node.isConnected)).toBe(true);
  await expect(page.locator('.tile[role="button"]')).toHaveCount(0);
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
    if(width<=900){
      await fixedDockRows(page,'.views');
      await expect.poll(()=>dock.locator('.views').evaluate((nav)=>{
        const current=nav.querySelector('[aria-current="true"]')?.getBoundingClientRect(),style=getComputedStyle(nav,'::before');
        if(current===undefined)return false;
        const thumbLeft=nav.getBoundingClientRect().left+parseFloat(style.left)+new DOMMatrix(style.transform).m41;
        return Math.abs(thumbLeft-current.left)<1;
      })).toBe(true);
    }
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
  const review=dock.getByRole('button', { name: /^Review the \d+$/ });
  const reviewCount=(await review.innerText()).match(/\d+/)?.[0];
  if(reviewCount===undefined)throw new Error('the review count is missing');
  await review.click();
  await expect(dock.locator('.place-review .num')).toHaveText(`1 of ${reviewCount}`);
  await expect(dock.getByRole('heading', {level:2})).toHaveText('What needs a look');
  await dock.getByRole('button', { name: 'Done', exact: true }).click();
  await dock.getByRole('button', { name: 'More', exact: true }).click();
  await expect(page.getByRole('dialog', { name: 'What needs a look', exact: true })).toBeVisible();
  await page.keyboard.press('Escape');
  const lowerHandle=page.locator('[data-end-handle="lower"]'),beforeLower=await lowerHandle.locator('text').textContent(),handleBox=await lowerHandle.locator('circle').boundingBox();
  if(handleBox===null)throw new Error('the lower handle has no box');
  await page.mouse.move(handleBox.x+handleBox.width/2,handleBox.y+handleBox.height/2);await page.mouse.down();
  await page.mouse.move(handleBox.x+handleBox.width/2,handleBox.y+handleBox.height/2+32,{steps:6});
  await expect(lowerHandle.locator('text')).not.toHaveText(beforeLower??'');await page.keyboard.press('Escape');await page.mouse.up();
  await expect(lowerHandle.locator('text')).toHaveText(beforeLower??'');await expect(dock.getByRole('heading',{level:2})).toHaveText(/^A first placement:/);await expect(dock.getByRole('button',{name:'Undo',exact:true})).toHaveCount(0);
  const stage=page.locator('.three-stage'),chip=page.locator('.place-chip');
  async function choosePart(fragment:string){
    await stage.focus();await page.keyboard.press('Escape');
    for(let at=0;at<40;at++){
      await page.keyboard.press('ArrowDown');
      if(await chip.count()&&(await chip.innerText()).split('\n')[0]?.includes(fragment)===true){await page.keyboard.press('Enter');return;}
    }
    throw new Error(`No part named ${fragment}`);
  }
  await choosePart('· screw');
  await expect(chip.getByRole('navigation',{name:'Part',exact:true})).toBeVisible();
  await page.screenshot({path:info.outputPath('screw-controls.png')});
  await chip.getByRole('button',{name:'Hook',exact:true}).click();
  await expect(dock.getByRole('heading',{level:2})).toHaveText(/^Your placement:/,{timeout:60_000});
  await expect(chip).toContainText('· hook');
  await choosePart('· screw');
  await chip.getByRole('button',{name:'Larger diameter',exact:true}).click();
  await expect(chip).toContainText('Set by hand',{timeout:60_000});
  await expect(chip).not.toContainText('Also ');
  await stage.focus();await page.keyboard.press('Escape');
  for(let at=0;at<40;at++){await page.keyboard.press('ArrowDown');if(await chip.count()&&(await chip.innerText()).startsWith('Lower end'))break;}
  const beforeEnd=await chip.innerText();await page.keyboard.press('Shift+ArrowDown');
  await expect.poll(()=>chip.innerText()).not.toBe(beforeEnd);
  await expect(dock.getByRole('heading',{level:2})).toHaveText(/^Your placement:/);
  for(let at=0;at<3;at++)await dock.getByRole('button',{name:'Undo',exact:true}).click();
  await expect(dock.getByRole('heading',{level:2})).toHaveText(/^A first placement:/);
  // Another hand change gives the load the placement currently on the stage.
  await choosePart('· screw');await chip.getByRole('button',{name:'Hook',exact:true}).click();
  await expect(dock.getByRole('heading',{level:2})).toHaveText(/^Your placement:/,{timeout:60_000});
  await dock.getByRole('button', { name: 'Put it under load', exact: true }).click();
  await expect(dock.getByRole('button', { name: 'Stop', exact: true })).toBeVisible();
  await expect(dock.getByRole('button', { name: 'Stop', exact: true })).toBeFocused();
  await dock.getByRole('button', { name: 'Stop', exact: true }).click();
  await expect(dock.locator('.say')).toContainText('Stopped. Nothing was kept.');
  await expect(dock.getByRole('button',{name:'Put it under load',exact:true})).toBeFocused();
  await dock.getByRole('button', { name: 'Put it under load', exact: true }).click();
  await expect(dock.getByRole('progressbar')).toBeVisible();
  const stops=dock.getByRole('navigation', { name: 'Load stop', exact: true });
  await expect(stops).toBeVisible({timeout:60_000});
  await expect(stops.getByRole('button',{name:'Rods in',exact:true})).toBeFocused();
  await expect(stops.getByRole('button')).toHaveText(['Before','Rods in']);
  for(const name of ['Standing','Bending forward','A year of walking'])await expect(stops.getByRole('button',{name,exact:true})).toHaveCount(0);
  let beforePoint: string|null=null;
  for(const [name,key] of [['Before','before'],['Rods in','rods-in']] as const){
    await stops.getByRole('button',{name,exact:true}).click();
    await expect(page.locator('.place-drawing')).toHaveAttribute('data-load-stop',key);
    await expect(stops.getByRole('button',{name,exact:true})).toHaveAttribute('aria-current','true');
    if(key==='before'){
      await expect(dock.getByRole('heading',{level:2})).toHaveText('Before: the spine as fitted.');
      await expect(page.locator('[data-range]')).toHaveCount(0);
      await page.waitForTimeout(700);
      beforePoint=await page.locator('[data-anchor="4"]').getAttribute('transform');
    }else{
      await expect(dock.getByRole('heading',{level:2})).toHaveText(new RegExp('^'+name+': the main curve is'));
      await expect.poll(()=>page.locator('[data-anchor="4"]').getAttribute('transform')).not.toBe(beforePoint);
      expect(await page.locator('[data-range]').count()).toBeLessThanOrEqual(3);
    }
    await expect.poll(()=>escapedDockText(page)).toEqual([]);
    await page.waitForTimeout(900);
    await page.screenshot({path:info.outputPath(`loaded-${key}.png`)});
  }
  for(const [width,height] of [[900,800],[640,800]] as const){
    await page.setViewportSize({width,height});
    await stops.getByRole('button',{name:'Rods in',exact:true}).click();
    await expect.poll(()=>escapedDockText(page)).toEqual([]);
    await fixedDockRows(page,'.load-stops');
    if(width<900)await expect(page.locator('.load-label-group').first()).not.toBeVisible();
    await page.waitForTimeout(400);
    await page.screenshot({path:info.outputPath(`loaded-rods-in-${String(width)}.png`)});
  }
  await page.setViewportSize({width:1440,height:900});
  await dock.getByRole('button',{name:'All estimates',exact:true}).click();
  await expect(page.getByRole('heading',{name:'All estimates',exact:true})).toBeVisible();
  await expect(page.locator('.load-estimate-list')).toContainText('Heaviest anchor');
  await expect(page.locator('.load-estimate-list .load-range').first()).toBeVisible();
  await expect(page.locator('.load-estimates')).toContainText("In this model standing, bending forward and walking add little to the forces locked in when the rods go in. The model has no muscles, and its figures for these loads come from adults; none measured in a child's fused spine was found.");
  await expect(page.locator('.load-estimates')).toContainText('The model does not rebalance the trunk above and below the rods.');
  await expect(page.locator('.load-estimates')).not.toContainText(/Weight not given|Steps a day not given|million steps|rod swing/i);
  await page.screenshot({path:info.outputPath('all-estimates.png')});
  await page.keyboard.press('Escape');
  const loadedSentence=await dock.getByRole('heading',{level:2}).innerText();
  const comparisonStarted=await page.evaluate(()=>performance.now());
  await dock.getByRole('button',{name:'Compare placements',exact:true}).click();
  await expect(page.locator('.place-column')).toHaveCount(4);
  await expect(page.locator('.tile').first()).not.toBeVisible();
  await expect(page.locator('html')).not.toHaveAttribute('data-links');
  await expect(page.locator('.place-column').first().getByRole('heading')).toHaveText('Yours');
  await expect.poll(()=>page.locator('.place-column[data-finished="true"]').count(),{timeout:120_000}).toBeGreaterThanOrEqual(3);
  await dock.getByRole('button',{name:'Heaviest anchor',exact:true}).click();
  await expect(dock.getByRole('button',{name:'Heaviest anchor',exact:true})).toHaveAttribute('aria-current','true');
  await expect(dock.getByRole('button',{name:'Stop',exact:true})).toHaveCount(0,{timeout:120_000});
  const comparisonMs=await page.evaluate((start)=>performance.now()-start,comparisonStarted);
  await info.attach('comparison-time',{body:`All comparison placements completed in ${String(comparisonMs)} ms.`,contentType:'text/plain'});
  await expect(page.locator('.place-column').first()).toContainText('%');
  await expect(page.locator('.place-comparison')).toHaveAttribute('data-settled','true');
  await page.screenshot({path:info.outputPath('comparison-yours.png')});
  await page.setViewportSize({width:640,height:800});
  await expect(page.locator('.place-column')).toHaveCount(2);
  await expect(page.locator('.place-comparison')).toHaveAttribute('data-settled','true');
  await expect.poll(()=>escapedDockText(page)).toEqual([]);
  await page.screenshot({path:info.outputPath('comparison-640.png')});
  await dock.getByRole('button',{name:'More placements',exact:true}).click();
  await expect(dock.getByRole('button',{name:'Earlier',exact:true})).toBeVisible();
  await dock.getByRole('button',{name:'Done',exact:true}).click();
  await expect(stops).toBeVisible();
  await expect(page.locator('html')).not.toHaveAttribute('data-placement-comparison');
  await expect(dock.getByRole('heading',{level:2})).toHaveText(loadedSentence);
  await page.setViewportSize({width:1440,height:900});
  await dock.getByRole('button',{name:'Compare placements',exact:true}).click();
  await expect(page.locator('.place-column').first()).toBeVisible();
  await steps.getByRole('button',{name:'3D',exact:true}).click();
  await expect(page.locator('#measurements')).toBeVisible();
  await expect(page.locator('#measurements')).toHaveText(fittedMeasurements,{useInnerText:true});
  await expect(page.locator('.place-comparison')).toHaveCount(0);
  await expect(page.locator('html')).not.toHaveAttribute('data-placement-comparison');
  await expect.poll(()=>page.locator('.dock').evaluate((dock)=>Math.abs(dock.getBoundingClientRect().width-(document.querySelector('.bar')?.getBoundingClientRect().width??0)))).toBeLessThan(1);
  await page.waitForTimeout(700);
  await page.screenshot({path:info.outputPath('three-after-comparison.png')});
  await steps.getByRole('button',{name:'Place',exact:true}).click();
  await expect(dock.getByRole('button',{name:'Compare placements',exact:true})).toBeVisible();
  await dock.getByRole('button',{name:'All estimates',exact:true}).click();
  const age=page.getByLabel('Age',{exact:true});await age.scrollIntoViewIfNeeded();
  await age.selectOption('9');
  await expect(dock.locator('.say')).toContainText('The age changed, so the placement was worked out again.',{timeout:60_000});
  await expect(dock.getByRole('button',{name:'Put it under load',exact:true})).toBeVisible();
  await expect(dock.getByRole('button',{name:'Undo',exact:true})).toHaveCount(0);
  await steps.getByRole('button', { name: '3D', exact: true }).click();
  await expect(page.locator('html')).toHaveAttribute('data-moment', 'three');
  await expect(canvas).toBeVisible();
  expect(await original.evaluate((node) => node.isConnected)).toBe(true);
  await expect(page.locator('#measurements')).toBeVisible();
  await expect(page.locator('.tile[role="button"]')).toHaveCount(4);
  await expect(page.locator('.place-drawing')).toHaveCount(0);
  await expect(caption).toHaveCount(0);
});
