// CC0 pixel art (Kenney "Tiny Dungeon", credited in CREDITS.md). 16x16 source tiles;
// the renderer scales them with imageSmoothingEnabled=false to stay crisp at any tile size.
import heroUrl from '../assets/sprites/hero.png';
import runnerUrl from '../assets/sprites/runner.png';
import walkerUrl from '../assets/sprites/walker.png';
import { loadSheets } from './tilemap';

function img(src: string): HTMLImageElement {
  const el = new Image();
  el.src = src;
  return el;
}

export const sprites = {
  hero: img(heroUrl),
  walker: img(walkerUrl),
  runner: img(runnerUrl),
};

/** Resolve once every sprite has decoded (or failed) — call before the first draw. */
export function loadSprites(): Promise<void> {
  return Promise.all([
    loadSheets(),
    ...Object.values(sprites).map(
      (el) =>
        new Promise<void>((resolve) => {
          if (el.complete) resolve();
          else {
            el.onload = () => resolve();
            el.onerror = () => resolve();
          }
        }),
    ),
  ]).then(() => undefined);
}
