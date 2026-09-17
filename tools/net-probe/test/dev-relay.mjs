import { WebSocketServer } from 'ws';
const wss = new WebSocketServer({ port: 7777 });
const subs = new Map(); // ws -> Map(subId, filter)
wss.on('connection', (ws) => {
  subs.set(ws, new Map());
  ws.on('message', (raw) => {
    const m = JSON.parse(raw);
    if (m[0] === 'REQ') subs.get(ws).set(m[1], m[2]);
    else if (m[0] === 'CLOSE') subs.get(ws).delete(m[1]);
    else if (m[0] === 'EVENT') {
      const ev = m[1];
      ws.send(JSON.stringify(['OK', ev.id, true, '']));
      const x = ev.tags.find((t) => t[0] === 'x')?.[1];
      for (const [c, map] of subs) for (const [sid, f] of map)
        if ((!f.kinds || f.kinds.includes(ev.kind)) && (!f['#x'] || f['#x'].includes(x))) c.send(JSON.stringify(['EVENT', sid, ev]));
    }
  });
  ws.on('close', () => subs.delete(ws));
});
console.log('relay up');
