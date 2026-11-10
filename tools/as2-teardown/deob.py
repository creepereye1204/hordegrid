"""Deobfuscating AS2 decompiler for control-flow-flattened SWFs.

Emulates bytecode with a mixed concrete/symbolic stack:
- concrete values (numbers, strings, bools, undefined) are folded, so the
  state-machine dispatcher, opaque predicates and junk ops evaluate away;
- anything that depends on runtime data becomes an expression string and
  side-effecting ops emit statements.
Branches on symbolic conditions fork; (pc, concrete-env) pairs are visited once
and become labels. Output: labelled pseudo-ActionScript per class.

Usage: python3 -I deob.py <swf> <outdir>
"""
import sys, struct, os, zlib, re
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import as2dis as D

UNDEF = object()
class E(str):
    """symbolic expression"""

def lit(v):
    if isinstance(v, E): return str(v)
    if v is UNDEF: return 'undefined'
    if v is None: return 'null'
    if v is True: return 'true'
    if v is False: return 'false'
    if isinstance(v, float):
        if v == int(v) and abs(v) < 1e15: return str(int(v))
        return repr(round(v, 6))
    if isinstance(v, int): return str(v)
    return repr(v)

def num(v):
    if v is True: return 1.0
    if v is False or v is None: return 0.0
    if v is UNDEF: return float('nan')
    if isinstance(v, (int, float)): return float(v)
    try: return float(v)
    except: return float('nan')

def truthy(v):
    if v is UNDEF or v is None: return False
    if isinstance(v, bool): return v
    if isinstance(v, (int, float)): return v == v and v != 0
    return len(v) > 0

def ident(s):
    return isinstance(s, str) and not isinstance(s, E)

def mem(o, m):
    os_ = lit(o)
    if ident(m) and re.fullmatch(r'[A-Za-z_$][\w$]*', m): return E(f'{os_}.{m}')
    return E(f'{os_}[{lit(m)}]')

def decode(code, pc):
    op = code[pc]
    if op >= 0x80:
        ln = struct.unpack_from('<H', code, pc + 1)[0]
        return op, code[pc+3:pc+3+ln], pc + 3 + ln
    return op, b'', pc + 1

def parse_push(p, pool, regs, regnames):
    k = 0; vals = []
    while k < len(p):
        t = p[k]; k += 1
        if t == 0: s, k = D.cstr(p, k); vals.append(s)
        elif t == 1: vals.append(float(struct.unpack_from('<f', p, k)[0])); k += 4
        elif t == 2: vals.append(None)
        elif t == 3: vals.append(UNDEF)
        elif t == 4:
            r = p[k]; k += 1
            vals.append(regs.get(r, E(regnames.get(r, f'r{r}'))))
        elif t == 5: vals.append(bool(p[k])); k += 1
        elif t == 6:
            hi, lo = struct.unpack_from('<II', p, k); k += 8
            vals.append(struct.unpack('<d', struct.pack('<II', lo, hi))[0])
        elif t == 7: vals.append(float(struct.unpack_from('<i', p, k)[0])); k += 4
        elif t == 8: i = p[k]; k += 1; vals.append(pool[i] if i < len(pool) else E(f'C{i}'))
        elif t == 9: i = struct.unpack_from('<H', p, k)[0]; k += 2; vals.append(pool[i] if i < len(pool) else E(f'C{i}'))
        else: break
    return vals

BINS = {0x0A:'+',0x0B:'-',0x0C:'*',0x0D:'/',0x47:'+',0x48:'<',0x49:'==',0x3F:'%',0x60:'&',0x61:'|',0x62:'^',
        0x63:'<<',0x64:'>>',0x65:'>>>',0x66:'===',0x67:'>',0x0E:'==',0x0F:'<',0x10:'&&',0x11:'||',0x21:'+',
        0x13:'==',0x29:'<',0x68:'>',0x54:'instanceof'}

def fold(op, a, b):
    if isinstance(a, E) or isinstance(b, E): return None
    try:
        if op in (0x47, 0x21) and (isinstance(a, str) or isinstance(b, str)):
            return (a if isinstance(a, str) else lit(a)) + (b if isinstance(b, str) else lit(b))
        x, y = num(a), num(b)
        if op in (0x0A, 0x47): return x + y
        if op == 0x0B: return x - y
        if op == 0x0C: return x * y
        if op == 0x0D: return x / y if y else float('inf')
        if op == 0x3F: return x % y
        if op in (0x0E, 0x49, 0x66, 0x13): return (a == b) if op in (0x66, 0x13) else (x == y if not (isinstance(a,str) and isinstance(b,str)) else a == b)
        if op in (0x0F, 0x48, 0x29): return x < y
        if op in (0x67, 0x68): return x > y
        if op == 0x10: return truthy(a) and truthy(b)
        if op == 0x11: return truthy(a) or truthy(b)
        ix, iy = int(x) & 0xffffffff, int(y)
        if op == 0x60: return float(int(x) & int(y))
        if op == 0x61: return float(int(x) | int(y))
        if op == 0x62: return float(int(x) ^ int(y))
        if op == 0x63: return float((int(x) << (iy & 31)) & 0xffffffff)
        if op == 0x64: return float(int(x) >> (iy & 31))
        if op == 0x65: return float(ix >> (iy & 31))
    except Exception:
        return None
    return None

class Fn:
    def __init__(self, code, lo, hi, regnames, depth, consts):
        self.code, self.lo, self.hi, self.depth = code, lo, hi, depth
        self.regnames = regnames
        self.blocks = {}      # label -> list of lines
        self.order = []
        self.consts = consts  # name -> constant returned by stub functions
        self.locals = {}

    def run(self, pool, start=None):
        work = [(self.lo if start is None else start, (), {}, {}, tuple(pool))]
        seen = {}
        budget = 400000
        while work and budget > 0:
            pc, stack, regs, locs, pool = work.pop()
            key = (pc, stack if all(not isinstance(s, E) for s in stack) else len(stack),
                   tuple(sorted((k, lit(v)) for k, v in locs.items())),
                   tuple(sorted((k, lit(v)) for k, v in regs.items() if not isinstance(v, E))))
            if key in seen: continue
            label = f'L{len(seen)}'; seen[key] = label
            lines = []; self.blocks[label] = lines; self.order.append(label)
            st = list(stack); regs = dict(regs); locs = dict(locs); pool = list(pool)
            def pop(): return st.pop() if st else E('<?>')
            def goto(npc, s2=None):
                k2 = (npc, tuple(s2 if s2 is not None else st) if all(not isinstance(x, E) for x in (s2 if s2 is not None else st)) else len(s2 if s2 is not None else st),
                      tuple(sorted((k, lit(v)) for k, v in locs.items())),
                      tuple(sorted((k, lit(v)) for k, v in regs.items() if not isinstance(v, E))))
                if k2 in seen: lines.append(f'goto {seen[k2]};'); return
                # predict label name lazily: push and reference by key
                lines.append(('GOTO', k2))
                work.append((npc, tuple(s2 if s2 is not None else st), regs, dict(locs), tuple(pool)))
            steps = 0
            while True:
                budget -= 1; steps += 1
                if steps > 40000 or budget <= 0: lines.append('/* step limit */'); break
                if pc == self.hi and self.depth > 0: lines.append('return;'); break
                if not (0 <= pc < len(self.code)): lines.append(f'/* out of range pc {pc} */'); break
                op, p, npc = decode(self.code, pc)
                if op == 0: lines.append('return;'); break
                try:
                    if op == 0x88:
                        cnt = struct.unpack_from('<H', p, 0)[0]; k = 2; pool = []
                        for _ in range(cnt): s, k = D.cstr(p, k); pool.append(s)
                    elif op == 0x96: st.extend(parse_push(p, pool, regs, self.regnames))
                    elif op in BINS:
                        b = pop(); a = pop(); r = fold(op, a, b)
                        st.append(r if r is not None else E(f'({lit(a)} {BINS[op]} {lit(b)})'))
                    elif op == 0x12:
                        a = pop(); st.append(E(f'!{lit(a)}') if isinstance(a, E) else (not truthy(a)))
                    elif op == 0x17:
                        v = pop()
                        if isinstance(v, E) and '(' in v and not v.startswith('('): lines.append(f'{v};')
                    elif op == 0x1C:
                        n = pop()
                        if isinstance(n, E): st.append(E(f'eval({n})'))
                        elif not isinstance(n, str): st.append(UNDEF)
                        elif n in locs: st.append(locs[n])
                        else: st.append(E(n))
                    elif op in (0x1D, 0x3C):
                        v = pop(); n = pop()
                        if ident(n) and (n.startswith('\x01') or not v is None and not isinstance(v, E) and not n.isidentifier()):
                            locs[n] = v
                        elif ident(n) and not n.isidentifier():
                            locs[n] = v
                        else:
                            if ident(n): locs.pop(n, None)
                            lines.append(f'{"var " if op==0x3C else ""}{n if ident(n) else lit(n)} = {lit(v)};')
                    elif op == 0x41:
                        n = pop(); lines.append(f'var {n};') if ident(n) and n.isidentifier() else None
                    elif op == 0x4E:
                        m = pop(); o = pop()
                        if not isinstance(o, E) and not isinstance(o, str): st.append(UNDEF)
                        else: st.append(mem(o, m))
                    elif op == 0x4F:
                        v = pop(); m = pop(); o = pop()
                        lines.append(f'{mem(o, m)} = {lit(v)};')
                    elif op in (0x3D, 0x40):
                        n = pop(); c = pop(); c = int(num(c)) if not isinstance(c, E) else 0
                        args = [pop() for _ in range(max(0, min(c, 64)))]
                        if not (op == 0x3D and ident(n) and n in self.consts):
                            st.append(E(f'{"new " if op==0x40 else ""}{n if ident(n) else lit(n)}({", ".join(lit(a) for a in args)})'))
                        else:
                            st.append(self.consts[n])
                    elif op in (0x52, 0x53):
                        m = pop(); o = pop(); c = pop(); c = int(num(c)) if not isinstance(c, E) else 0
                        args = [pop() for _ in range(max(0, min(c, 64)))]
                        callee = lit(o) if (m is UNDEF or m == '') else mem(o, m)
                        st.append(E(f'{"new " if op==0x53 else ""}{callee}({", ".join(lit(a) for a in args)})'))
                        if not isinstance(o, E) and not isinstance(o, str): st[-1] = UNDEF
                    elif op == 0x42:
                        c = int(num(pop())); st.append(E('[' + ', '.join(lit(pop()) for _ in range(c)) + ']'))
                    elif op == 0x43:
                        c = int(num(pop())); kv = []
                        for _ in range(c): v = pop(); k = pop(); kv.append(f'{k if ident(k) else lit(k)}: {lit(v)}')
                        st.append(E('{' + ', '.join(kv) + '}'))
                    elif op == 0x87:
                        r = p[0]; v = st[-1] if st else E('<?>')
                        if isinstance(v, E) and not (r in self.regnames) and ('\n' in v or len(v) > 60 or '(' in v):
                            lines.append(f'var r{r} = {v};'); regs[r] = E(f'r{r}'); st[-1] = regs[r]
                        elif isinstance(v, E) and not (r in self.regnames):
                            regs[r] = v
                        elif r in self.regnames:
                            lines.append(f'{self.regnames[r]} = {lit(v)};'); regs.pop(r, None)
                        else:
                            regs[r] = v
                    elif op == 0x4C: st.append(st[-1] if st else E('<?>'))
                    elif op == 0x4D:
                        if len(st) >= 2: st[-1], st[-2] = st[-2], st[-1]
                    elif op in (0x50, 0x51):
                        a = pop(); d = 1 if op == 0x50 else -1
                        st.append(E(f'({lit(a)} {"+" if d>0 else "-"} 1)') if isinstance(a, E) else num(a) + d)
                    elif op == 0x3E: lines.append(f'return {lit(pop())};'); break
                    elif op == 0x44:
                        a = pop(); st.append(E(f'typeof({lit(a)})'))
                    elif op in (0x4A, 0x18):
                        a = pop(); st.append(E(f'{"Number" if op==0x4A else "int"}({lit(a)})') if isinstance(a, E) else (float(int(num(a))) if op==0x18 else num(a)))
                    elif op == 0x4B: a = pop(); st.append(E(f'String({lit(a)})'))
                    elif op == 0x30: st.append(E(f'random({lit(pop())})'))
                    elif op == 0x34: st.append(E('getTimer()'))
                    elif op == 0x26: lines.append(f'trace({lit(pop())});')
                    elif op == 0x3A: m = pop(); o = pop(); lines.append(f'delete {mem(o, m)};')
                    elif op == 0x3B: lines.append(f'delete {lit(pop())};')
                    elif op == 0x69: s = pop(); c = pop(); lines.append(f'{lit(c)} extends {lit(s)};')
                    elif op == 0x2C:
                        o = pop(); c = int(num(pop())); ifs = [lit(pop()) for _ in range(c)]
                        lines.append(f'{lit(o)} implements {", ".join(ifs)};')
                    elif op == 0x2B: o = pop(); c = pop(); st.append(E(f'({lit(c)}){lit(o)}'))
                    elif op in (0x46, 0x55):
                        o = pop(); st.append(None); st.append(E(f'nextKey({lit(o)})'))
                        lines.append(f'/* for-in over {lit(o)} */')
                    elif op == 0x2A: lines.append(f'throw {lit(pop())};'); break
                    elif op == 0x99:
                        off = struct.unpack_from('<h', p, 0)[0]; goto(npc + off); break
                    elif op == 0x9D:
                        off = struct.unpack_from('<h', p, 0)[0]; c = pop()
                        if isinstance(c, E):
                            t = npc + off
                            lines.append(('IF', lit(c)))
                            goto(t); goto(npc); break
                        if truthy(c): pc = npc + off
                        else: pc = npc
                        continue
                    elif op in (0x9B, 0x8E):
                        k = 0; fname, k = D.cstr(p, k); np_ = struct.unpack_from('<H', p, k)[0]; k += 2
                        rn = {}; ps = []
                        if op == 0x8E:
                            k += 1; fl = struct.unpack_from('<H', p, k)[0]; k += 2
                            for _ in range(np_):
                                reg = p[k]; k += 1; s, k = D.cstr(p, k); ps.append(s)
                                if reg: rn[reg] = s
                            r = 1
                            for bit, name in ((0x01,'this'),(0x04,'arguments'),(0x10,'super'),(0x40,'_root'),(0x80,'_parent'),(0x100,'_global')):
                                if fl & bit: rn[r] = name; r += 1
                        else:
                            for _ in range(np_): s, k = D.cstr(p, k); ps.append(s)
                        sz = struct.unpack_from('<H', p, k)[0]
                        body_lo, body_hi = npc, npc + sz
                        # stub "function(){ return CONST; }" used by the dispatcher seed
                        b_op, b_p, b_n = decode(self.code, body_lo)
                        stub = None
                        if b_op == 0x96 and decode(self.code, b_n)[0] == 0x3E and b_n + 1 == body_hi:
                            vals = parse_push(b_p, pool, {}, {})
                            if len(vals) == 1 and not isinstance(vals[0], E): stub = vals[0]
                        if stub is not None and fname:
                            self.consts[fname] = stub
                        else:
                            sub = Fn(self.code, body_lo, body_hi, rn, self.depth + 1, dict(self.consts))
                            sub.run(pool, body_lo)
                            text = sub.render()
                            sig = f'function {fname}({", ".join(ps)})'
                            if fname:
                                lines.append(sig + ' {'); lines.extend('  ' + l for l in text); lines.append('}')
                            else:
                                st.append(E(sig + ' {\n' + '\n'.join('  ' + l for l in text) + '\n}'))
                        pc = body_hi; continue
                    elif op == 0x8F: lines.append('/* try */')
                    elif op == 0x94: lines.append(f'with ({lit(pop())}) /* ... */')
                    elif op in (0x22,): pr = pop(); t = pop(); st.append(E(f'getProperty({lit(t)}, {lit(pr)})'))
                    elif op in (0x23,): v = pop(); pr = pop(); t = pop(); lines.append(f'setProperty({lit(t)}, {lit(pr)}, {lit(v)});')
                    elif op in (0x06, 0x07): lines.append(D.OP[op] + '();')
                    elif op == 0x20: lines.append(f'tellTarget({lit(pop())});')
                    elif op == 0x45: st.append(E(f'targetPath({lit(pop())})'))
                    else: lines.append(f'/* {D.OP.get(op, hex(op))} */')
                except Exception as e:
                    lines.append(f'/* err {type(e).__name__}: {e} @ {pc} */'); break
                pc = npc
        self.seen = seen
        return self

    def render(self):
        inv = getattr(self, 'seen', {})
        out = []; used = set()
        for lab in self.order:
            for l in self.blocks[lab]:
                if isinstance(l, tuple) and l[0] == 'GOTO': used.add(inv.get(l[1]))
        for n, lab in enumerate(self.order):
            blk = self.blocks[lab]
            if lab in used or n > 0: out.append(f'{lab}:')
            pend_if = None
            for l in blk:
                if isinstance(l, tuple):
                    if l[0] == 'IF': pend_if = l[1]; continue
                    tgt = inv.get(l[1], '?')
                    if pend_if is not None: out.append(f'  if ({pend_if}) goto {tgt};'); pend_if = None
                    else: out.append(f'  goto {tgt};')
                else:
                    out.append('  ' + l if not l.startswith('  ') else l)
        # collapse "L: goto M;" trivia is left as-is for traceability
        return out

def locate(b):
    nb = b[8] >> 3; S = 8 + (5 + 4 * nb + 7) // 8 + 4
    loc = {}
    def walk(s, e):
        r = D.R(b, s)
        while r.i < e:
            h = r.u16(); code = h >> 6; ln = h & 0x3f
            if ln == 0x3f: ln = r.u32()
            if code == 59: loc[struct.unpack_from('<H', b, r.i)[0]] = (r.i + 2, r.i + ln)
            if code == 39: walk(r.i + 4, r.i + ln)
            r.i += ln
            if code == 0: return
    walk(S, len(b))
    return loc

def main():
    path, outd = sys.argv[1], sys.argv[2]
    raw = open(path, 'rb').read()
    b = raw[:8] + (zlib.decompress(raw[8:]) if raw[:3] == b'CWS' else raw[8:])
    os.makedirs(outd, exist_ok=True)
    only = sys.argv[3].split(',') if len(sys.argv) > 3 else None
    loc = locate(b); n = 0
    for sid, (lo, hi) in sorted(loc.items()):
        if only and str(sid) not in only: continue
        f = Fn(b, lo, hi, {}, 0, {}).run([], lo)
        txt = '\n'.join(f.render())
        m = re.findall(r'_global\.([\w.]+) = ', txt)
        name = max(m, key=len) if m else f'init_{sid}'
        open(os.path.join(outd, f'{sid}_{name}.as'), 'w').write(txt); n += 1
    print(n, 'blocks')

def _old_main():
    path, outd = sys.argv[1], sys.argv[2]
    raw = open(path, 'rb').read()
    b = raw[:8] + (zlib.decompress(raw[8:]) if raw[:3] == b'CWS' else raw[8:])
    nb = b[8] >> 3; i = 8 + (5 + 4 * nb + 7) // 8 + 4
    os.makedirs(outd, exist_ok=True)
    only = sys.argv[3] if len(sys.argv) > 3 else None
    names = {}
    def walk(start, end):
        for code, body in D.tags(b, start, end):
            if code == 59:
                sid = struct.unpack_from('<H', body, 0)[0]
                if only and str(sid) != only: continue
                c = body[2:]
                f = Fn(c, 0, len(c), {}, 0, {}).run([])
                txt = '\n'.join(f.render())
                m = re.findall(r'_global\.([\w.]+) = function', txt)
                name = max(m, key=len) if m else f'init_{sid}'
                names[sid] = name
                open(os.path.join(outd, f'{sid}_{name}.as'), 'w').write(txt)
            elif code == 39:
                off = b.find(body); walk(off + 4, off + len(body))
    walk(i, len(b))
    print(len(names), 'classes')

if __name__ == '__main__':
    main()
