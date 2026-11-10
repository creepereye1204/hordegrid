"""Minimal SWF (v<=8) tag walker + ActionScript 1/2 disassembler.
Usage: python3 -I as2dis.py <swf> <outdir>   (raw disassembly, one .asm per action block)
Writes one .asm file per action block, named by owner (sprite id / export name / frame).
"""
import sys, zlib, struct, os

OP = {
0x00:'end',0x04:'nextFrame',0x05:'prevFrame',0x06:'play',0x07:'stop',0x08:'toggleQuality',0x09:'stopSounds',
0x0A:'add',0x0B:'subtract',0x0C:'multiply',0x0D:'divide',0x0E:'equals',0x0F:'less',0x10:'and',0x11:'or',0x12:'not',
0x13:'stringEquals',0x14:'stringLength',0x15:'stringExtract',0x17:'pop',0x18:'toInteger',0x1C:'getVariable',0x1D:'setVariable',
0x20:'setTarget2',0x21:'stringAdd',0x22:'getProperty',0x23:'setProperty',0x24:'cloneSprite',0x25:'removeSprite',0x26:'trace',
0x27:'startDrag',0x28:'endDrag',0x29:'stringLess',0x2A:'throw',0x2B:'castOp',0x2C:'implementsOp',0x30:'randomNumber',
0x31:'mbStringLength',0x32:'charToAscii',0x33:'asciiToChar',0x34:'getTime',0x35:'mbStringExtract',0x36:'mbCharToAscii',
0x37:'mbAsciiToChar',0x3A:'delete',0x3B:'delete2',0x3C:'defineLocal',0x3D:'callFunction',0x3E:'return',0x3F:'modulo',
0x40:'newObject',0x41:'defineLocal2',0x42:'initArray',0x43:'initObject',0x44:'typeOf',0x45:'targetPath',0x46:'enumerate',
0x47:'add2',0x48:'less2',0x49:'equals2',0x4A:'toNumber',0x4B:'toString',0x4C:'pushDuplicate',0x4D:'stackSwap',
0x4E:'getMember',0x4F:'setMember',0x50:'increment',0x51:'decrement',0x52:'callMethod',0x53:'newMethod',0x54:'instanceOf',
0x55:'enumerate2',0x60:'bitAnd',0x61:'bitOr',0x62:'bitXor',0x63:'bitLShift',0x64:'bitRShift',0x65:'bitURShift',
0x66:'strictEquals',0x67:'greater',0x68:'stringGreater',0x69:'extends',
0x81:'gotoFrame',0x83:'getURL',0x87:'storeRegister',0x88:'constantPool',0x8A:'waitForFrame',0x8B:'setTarget',0x8C:'gotoLabel',
0x8D:'waitForFrame2',0x8E:'defineFunction2',0x8F:'try',0x94:'with',0x96:'push',0x99:'jump',0x9A:'getURL2',0x9B:'defineFunction',
0x9D:'if',0x9E:'call',0x9F:'gotoFrame2',
}

def cstr(b, i):
    j = b.index(0, i)
    return b[i:j].decode('latin-1'), j + 1

def disasm(code, pool_in=None):
    out = []; i = 0; pool = list(pool_in or [])
    n = len(code)
    while i < n:
        at = i; op = code[i]; i += 1
        if op == 0: out.append(f'{at:06d} end'); continue
        name = OP.get(op, f'op_{op:02x}')
        if op < 0x80:
            out.append(f'{at:06d} {name}'); continue
        ln = struct.unpack_from('<H', code, i)[0]; i += 2
        p = code[i:i+ln]; i += ln
        arg = ''
        try:
            if op == 0x88:
                cnt = struct.unpack_from('<H', p, 0)[0]; k = 2; pool = []
                for _ in range(cnt):
                    s, k = cstr(p, k); pool.append(s)
                arg = f'[{len(pool)}] ' + ' | '.join(pool)
            elif op == 0x96:
                k = 0; vals = []
                while k < len(p):
                    t = p[k]; k += 1
                    if t == 0: s, k = cstr(p, k); vals.append(repr(s))
                    elif t == 1: vals.append(str(struct.unpack_from('<f', p, k)[0])); k += 4
                    elif t == 2: vals.append('null')
                    elif t == 3: vals.append('undefined')
                    elif t == 4: vals.append(f'r{p[k]}'); k += 1
                    elif t == 5: vals.append('true' if p[k] else 'false'); k += 1
                    elif t == 6:
                        hi, lo = struct.unpack_from('<II', p, k); k += 8
                        vals.append(repr(struct.unpack('<d', struct.pack('<II', lo, hi))[0]))
                    elif t == 7: vals.append(str(struct.unpack_from('<i', p, k)[0])); k += 4
                    elif t == 8: idx = p[k]; k += 1; vals.append(repr(pool[idx]) if idx < len(pool) else f'c{idx}')
                    elif t == 9: idx = struct.unpack_from('<H', p, k)[0]; k += 2; vals.append(repr(pool[idx]) if idx < len(pool) else f'c{idx}')
                    else: vals.append(f'?t{t}'); break
                arg = ', '.join(vals)
            elif op in (0x99, 0x9D):
                off = struct.unpack_from('<h', p, 0)[0]; arg = f'-> {i+off:06d}'
            elif op == 0x87: arg = f'r{p[0]}'
            elif op in (0x83,):
                a, k = cstr(p, 0); b, k = cstr(p, k); arg = f'{a!r} {b!r}'
            elif op == 0x8C: arg = repr(cstr(p, 0)[0])
            elif op == 0x81: arg = str(struct.unpack_from('<H', p, 0)[0])
            elif op == 0x9B:
                fname, k = cstr(p, 0); np_ = struct.unpack_from('<H', p, k)[0]; k += 2; ps = []
                for _ in range(np_):
                    s, k = cstr(p, k); ps.append(s)
                sz = struct.unpack_from('<H', p, k)[0]
                arg = f'{fname or "<anon>"}({", ".join(ps)}) size={sz}'
            elif op == 0x8E:
                fname, k = cstr(p, 0); np_ = struct.unpack_from('<H', p, k)[0]; k += 2
                regc = p[k]; k += 1; k += 2; ps = []
                for _ in range(np_):
                    reg = p[k]; k += 1; s, k = cstr(p, k); ps.append(f'{s}@r{reg}' if reg else s)
                sz = struct.unpack_from('<H', p, k)[0]
                arg = f'{fname or "<anon>"}({", ".join(ps)}) regs={regc} size={sz}'
            elif op == 0x8B: arg = repr(cstr(p, 0)[0])
            else: arg = p.hex()[:40]
        except Exception as e:
            arg = f'<parse err {e}>'
        out.append(f'{at:06d} {name} {arg}')
    return out

class R:
    def __init__(s, b, i=0): s.b = b; s.i = i
    def u8(s): v = s.b[s.i]; s.i += 1; return v
    def u16(s): v = struct.unpack_from('<H', s.b, s.i)[0]; s.i += 2; return v
    def u32(s): v = struct.unpack_from('<I', s.b, s.i)[0]; s.i += 4; return v

def tags(b, start, end):
    r = R(b, start)
    while r.i < end:
        h = r.u16(); code = h >> 6; ln = h & 0x3f
        if ln == 0x3f: ln = r.u32()
        yield code, b[r.i:r.i+ln]
        r.i += ln
        if code == 0: return

def clip_actions(body, ver):
    """PlaceObject2 clip actions -> list of (event flags, code)."""
    r = R(body); flags = r.u8(); r.u16()  # depth
    if flags & 0x02: r.u16()              # char id
    if flags & 0x04:                      # matrix
        bb = body; bit = r.i * 8
        def rb(n):
            nonlocal bit
            v = 0
            for _ in range(n):
                v = (v << 1) | ((bb[bit >> 3] >> (7 - (bit & 7))) & 1); bit += 1
            return v
        if rb(1): nb = rb(5); rb(nb); rb(nb)
        if rb(1): nb = rb(5); rb(nb); rb(nb)
        nb = rb(5); rb(nb); rb(nb)
        r.i = (bit + 7) >> 3
    if flags & 0x08:                      # cxform with alpha
        bb = body; bit = r.i * 8
        def rb2(n):
            nonlocal bit
            v = 0
            for _ in range(n):
                v = (v << 1) | ((bb[bit >> 3] >> (7 - (bit & 7))) & 1); bit += 1
            return v
        ha = rb2(1); hm = rb2(1); nb = rb2(4)
        if hm: [rb2(nb) for _ in range(4)]
        if ha: [rb2(nb) for _ in range(4)]
        r.i = (bit + 7) >> 3
    if flags & 0x10: r.u16()
    name = None
    if flags & 0x20: name, r.i = cstr(body, r.i)
    if flags & 0x40: r.u16()
    res = []
    if flags & 0x80:
        r.u16(); r.u32() if ver >= 6 else r.u16()
        while True:
            ev = r.u32() if ver >= 6 else r.u16()
            if ev == 0: break
            sz = r.u32()
            if ev & 0x00020000 if ver >= 6 else False: r.u8(); sz -= 1  # keypress
            res.append((ev, body[r.i:r.i+sz])); r.i += sz
    return name, res

def main():
    path, outd = sys.argv[1], sys.argv[2]
    raw = open(path, 'rb').read()
    sig, ver = raw[:3], raw[3]
    b = raw[:8] + (zlib.decompress(raw[8:]) if sig == b'CWS' else raw[8:])
    nb = b[8] >> 3; rect_bytes = (5 + 4 * nb + 7) // 8
    i = 8 + rect_bytes + 4
    os.makedirs(outd, exist_ok=True)
    exports = {}; blocks = []
    def walk(start, end, owner, frame0):
        frame = frame0
        for code, body in tags(b, start, end):
            if code == 1: frame += 1
            elif code == 12: blocks.append((f'{owner}_frame{frame}', body))
            elif code == 59:
                sid = struct.unpack_from('<H', body, 0)[0]; blocks.append((f'init_{sid}', body[2:]))
            elif code == 39:
                sid = struct.unpack_from('<H', body, 0)[0]
                off = b.find(body); walk(off + 4, off + len(body), f'sprite{sid}', 1)
            elif code == 56:
                r = R(body); n = r.u16()
                for _ in range(n):
                    cid = r.u16(); s, r.i = cstr(body, r.i); exports[cid] = s
            elif code == 26:
                try:
                    name, acts = clip_actions(body, ver)
                    for k, (ev, c) in enumerate(acts):
                        blocks.append((f'{owner}_f{frame}_clip_{name or "anon"}_ev{ev:x}_{k}', c))
                except Exception:
                    pass
    walk(i, len(b), 'root', 1)
    idx = []
    for name, code in blocks:
        for cid, ex in exports.items():
            name = name.replace(f'init_{cid}', f'init_{cid}_{ex}').replace(f'sprite{cid}_', f'sprite{cid}[{ex}]_')
        safe = ''.join(ch if ch.isalnum() or ch in '._-[]' else '_' for ch in name)[:150]
        lines = disasm(code)
        open(os.path.join(outd, safe + '.asm'), 'w').write('\n'.join(lines))
        idx.append(f'{safe}\t{len(code)}')
    open(os.path.join(outd, '_index.tsv'), 'w').write('\n'.join(idx))
    open(os.path.join(outd, '_exports.tsv'), 'w').write('\n'.join(f'{k}\t{v}' for k, v in sorted(exports.items())))
    print(f'swf v{ver}, {len(b)} bytes, {len(blocks)} action blocks, {len(exports)} exports')

if __name__ == "__main__": main()
