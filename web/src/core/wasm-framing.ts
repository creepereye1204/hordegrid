/** Split `take_outbox()` framing `[slot u8][len u16 LE][bytes]*`. */
export function* iterOutbox(buf: Uint8Array): Generator<[slot: number, bytes: Uint8Array]> {
  let i = 0;
  while (i + 3 <= buf.length) {
    const slot = buf[i]!;
    const len = buf[i + 1]! | (buf[i + 2]! << 8);
    i += 3;
    if (i + len > buf.length) return;
    yield [slot, buf.subarray(i, i + len)];
    i += len;
  }
}
