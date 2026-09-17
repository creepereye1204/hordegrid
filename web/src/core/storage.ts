/** Per-viewer conveniences only; every access may throw (private mode, blocked storage). */
export function load(key: string): string | null {
  try {
    return localStorage.getItem(`hordegrid:${key}`);
  } catch {
    return null;
  }
}

export function save(key: string, value: string): void {
  try {
    localStorage.setItem(`hordegrid:${key}`, value);
  } catch {
    /* storage unavailable: preference simply isn't remembered */
  }
}
