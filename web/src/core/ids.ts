/** Invite code: 8 digits (docs/design-docs/signaling.md#룸-모델). */
export const CODE_RE = /^\d{8}$/;

export function randomU32(): number {
  const a = new Uint32Array(1);
  crypto.getRandomValues(a);
  return a[0]!;
}

export function newInviteCode(): string {
  return String(randomU32() % 100_000_000).padStart(8, '0');
}

/** Extract digits from user input like "4821 0937" / "4821-0937". */
export function normalizeCode(input: string): string | null {
  const digits = input.replace(/\D/g, '');
  return CODE_RE.test(digits) ? digits : null;
}

export const formatCode = (code: string): string => `${code.slice(0, 4)} ${code.slice(4)}`;

const ADJ = ['용감한', '졸린', '빠른', '조용한', '배고픈', '수상한', '반짝이는', '느긋한'];
const NOUN = ['좀비사냥꾼', '생존자', '너구리', '경비원', '택배기사', '고양이', '탐험가', '요리사'];

export function randomNickname(): string {
  const r = randomU32();
  return `${ADJ[r % ADJ.length]}${NOUN[(r >>> 8) % NOUN.length]}-${(r >>> 16) % 100}`;
}

/** Clamp untrusted names: drop control characters, max 16 code points. */
export function sanitizeName(name: unknown): string {
  if (typeof name !== 'string') return '???';
  const chars = [...name].filter((c) => {
    const code = c.codePointAt(0) ?? 0;
    return code >= 32 && code !== 127;
  });
  const clean = chars.join('').trim();
  return clean ? [...clean].slice(0, 16).join('') : '???';
}

/** Session seed agreed by all: FNV-1a over sorted peer ids (docs/design-docs/determinism.md#난수). */
export function seedFromIds(ids: readonly string[], salt: number): number {
  let h = (0x811c9dc5 ^ salt) >>> 0;
  for (const id of [...ids].sort()) {
    for (let i = 0; i < id.length; i++) {
      h ^= id.charCodeAt(i);
      h = Math.imul(h, 0x01000193) >>> 0;
    }
  }
  return h >>> 0;
}
