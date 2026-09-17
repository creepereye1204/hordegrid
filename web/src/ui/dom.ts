// Thin DOM helpers. Untrusted strings are only ever assigned through textContent (docs/SECURITY.md).
export function $<T extends HTMLElement = HTMLElement>(id: string): T {
  const el = document.getElementById(id);
  if (!el) throw new Error(`#${id} missing`);
  return el as T;
}

const SCREENS = ['screen-loading', 'screen-title', 'screen-lobby', 'screen-over', 'screen-menu'] as const;
export type ScreenId = (typeof SCREENS)[number] | 'game';

export function showScreen(id: ScreenId): void {
  for (const s of SCREENS) $(s).hidden = s !== id;
  $('hud').hidden = id !== 'game' && id !== 'screen-over' && id !== 'screen-menu';
}

export function el<K extends keyof HTMLElementTagNameMap>(tag: K, props: { className?: string; text?: string } = {}, children: Node[] = []): HTMLElementTagNameMap[K] {
  const node = document.createElement(tag);
  if (props.className) node.className = props.className;
  if (props.text !== undefined) node.textContent = props.text;
  for (const c of children) node.append(c);
  return node;
}

/** Update text only when it changed (HUD runs every frame). */
export function setText(node: HTMLElement, text: string): void {
  if (node.textContent !== text) node.textContent = text;
}

let bannerTimer = 0;
export function banner(text: string, ms = 1500): void {
  const b = $('banner');
  b.textContent = text;
  b.hidden = false;
  window.clearTimeout(bannerTimer);
  bannerTimer = window.setTimeout(() => (b.hidden = true), ms);
}

export function toast(text: string, ms = 2500): void {
  const box = $('toasts');
  const t = el('div', { text });
  box.append(t);
  while (box.children.length > 3) box.firstElementChild?.remove();
  window.setTimeout(() => t.remove(), ms);
}

export const PLAYER_COLORS = ['var(--p1)', 'var(--p2)', 'var(--p3)', 'var(--p4)'] as const;
