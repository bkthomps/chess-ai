// UI wiring checks with a simulated DOM; not a visual browser test.
const fs = require('node:fs'), vm = require('node:vm'), assert = require('node:assert/strict'), path = require('node:path');
const html = fs.readFileSync(path.join(__dirname, '../web/index.html'), 'utf8');
let focused;
const audioNotes = [], storedSounds = new Map();
class AudioContextMock {
  constructor() { this.state = 'suspended'; this.currentTime = 0; this.destination = {}; }
  resume() { this.state = 'running'; return Promise.resolve(); }
  createOscillator() {
    const note = {};
    return {
      frequency: {setValueAtTime(value) { note.frequency = value; }, exponentialRampToValueAtTime(value) { note.endFrequency = value; }},
      connect() {}, disconnect() {},
      start(time) { note.time = time; note.type = this.type; audioNotes.push(note); },
      stop(time) { if (time === undefined) this.onended?.(); }
    };
  }
  createGain() {
    return {gain: {setValueAtTime() {}, exponentialRampToValueAtTime() {}}, connect() {}, disconnect() {}};
  }
}
class Element {
  constructor(fragment = false) {
    this.children = []; this.attrs = {}; this.dataset = {}; this.classes = new Set(); this.listeners = {};
    this.value = ''; this.hidden = false; this.disabled = false; this.fragment = fragment; this._html = '';
    this.style = {setProperty() {}};
  }
  get classList() { return {add: c => this.classes.add(c), remove: c => this.classes.delete(c), toggle: (c, on) => on ? this.classes.add(c) : this.classes.delete(c), contains: c => this.classes.has(c)}; }
  set className(v) { this.classes = new Set(v.split(' ')); }
  get className() { return [...this.classes].join(' '); }
  set innerHTML(v) { this._html = v; this.children = []; }
  get innerHTML() { return this._html; }
  append(e) { if (e.fragment) this.children.push(...e.children); else this.children.push(e); }
  replaceChildren(...args) { this.children = []; args.forEach(e => this.append(e)); }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  focus() { focused = this; }
  contains(e) { return this.children.includes(e); }
  querySelector(s) { const m = s.match(/data-square="(\d+)"/); return m ? this.children.find(e => String(e.dataset.square) === m[1]) : null; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  closest() { return this; }
  showModal() { this.open = true; }
  close() { this.open = false; this.listeners.close?.(); }
}
const elements = {};
for (const m of html.matchAll(/<[^>]*\bid="([^"]+)"[^>]*>/g)) {
  const e = elements[m[1]] = new Element(); e.hidden = /\bhidden\b/.test(m[0]);
  e.disabled = /\bdisabled\b/.test(m[0]); e.value = m[0].match(/\bvalue="([^"]+)"/)?.[1] || '';
}
elements['time-control'].value = '5';
elements['thinking-time'].value = '3';
let mode = 'two', color = 'white', server, requests = [], rejectNextMove = false;
const initial = [4,2,3,5,6,3,2,4,...Array(8).fill(1),...Array(32).fill(0),...Array(8).fill(-1),-4,-2,-3,-5,-6,-3,-2,-4];
const fresh = (human = null, base = 300, increment = 0) => ({
  board: [...initial], white: true, human, thinking_seconds: 3, revision: 1, thinking: false, check: false, result: null,
  legal: [[12,20,0],[12,28,0]], notation: [], last: null, stats: null,
  clocks: {enabled: true, white_ms: base * 1000, black_ms: base * 1000, running: true, active_white: true, base_seconds: base, increment_seconds: increment}
});
server = fresh(); server.clocks.running = false;
const context = vm.createContext({
  console, URLSearchParams, Set, AbortSignal, performance, clearTimeout, setTimeout: () => 0, setInterval: () => 0,
  window: {scrollTo() {}, confirm: () => true, AudioContext: AudioContextMock, localStorage: {getItem: key => storedSounds.get(key) ?? null, setItem: (key, value) => storedSounds.set(key, value)}},
  FormData: class {get(k) {return {mode, color, 'time-control': elements['time-control'].value, minutes: elements['base-minutes'].value, increment: elements['increment-seconds'].value, 'thinking-time': elements['thinking-time'].value, 'thinking-seconds': elements['thinking-seconds'].value}[k];}},
  document: {hidden: false, body: new Element(), getElementById: id => elements[id], createElement: () => new Element(), createDocumentFragment: () => new Element(true), get activeElement() {return focused;}, querySelector: () => ({value: mode}), querySelectorAll: () => [new Element(), new Element()]},
  fetch: async (url, options = {}) => {
    requests.push({url, options}); const u = new URL(url, 'http://localhost');
    if (u.pathname === '/api/move' && rejectNextMove) {
      rejectNextMove = false; return {ok: false, json: async () => ({error: 'That move is not legal.'})};
    }
    if (u.pathname === '/api/new') {
      server = fresh(u.searchParams.get('mode') === 'two' ? null : u.searchParams.get('color') === 'white', Number(u.searchParams.get('base_seconds')), Number(u.searchParams.get('increment_seconds')));
      if (u.searchParams.get('unlimited') === 'true') {
        server.thinking_seconds = Number(u.searchParams.get('thinking_seconds') || 3);
        server.clocks.enabled = false; server.clocks.running = false; server.clocks.active_white = null;
      }
      server.thinking = server.human === false;
    }
    if (u.pathname === '/api/move') {
      server.clocks[server.white ? 'white_ms' : 'black_ms'] += server.clocks.increment_seconds * 1000;
      server = {...server, revision: server.revision + 1, white: !server.white, notation: [...server.notation, 'e4'], last: [Number(u.searchParams.get('from')), Number(u.searchParams.get('to'))]};
      if (server.clocks.enabled) server.clocks.active_white = server.white;
    }
    if (u.pathname === '/api/undo') {
      server.notation.pop(); server.revision++; server.result = null; server.check = false;
    }
    if (u.pathname === '/api/resign') {
      server.result = 'Black wins by resignation'; server.revision++; server.clocks.running = false;
    }
    return {ok: true, json: async () => structuredClone(server)};
  }
});
vm.runInContext(fs.readFileSync(path.join(__dirname, '../web/app.js'), 'utf8'), context);
const tick = () => new Promise(r => setImmediate(r));
const submit = () => elements['setup-form'].listeners.submit({preventDefault() {}, currentTarget: elements['setup-form']});
const clickSquare = s => elements.board.listeners.click({target: elements.board.children.find(e => e.dataset.square === s)});
(async () => {
  await tick(); assert(!elements.menu.hidden); assert(elements.game.hidden);
  vm.runInContext('changeMode()', context); assert(elements['solo-options'].hidden);
  // Every preset must reach the server in seconds, including both increments.
  for (const [preset, base, increment] of [['1',60,0], ['2+1',120,1], ['5',300,0], ['10+5',600,5]]) {
    elements['time-control'].value = preset; elements['time-control'].listeners.change();
    assert(elements['custom-time'].hidden); assert(elements['base-minutes'].disabled);
    await submit(); const params = new URL(requests.at(-1).url, 'http://localhost').searchParams;
    assert.equal(Number(params.get('base_seconds')), base); assert.equal(Number(params.get('increment_seconds')), increment);
    assert.equal(elements.board.children.length, 64);
    elements['menu-button'].onclick();
  }
  elements['time-control'].value = '2+1'; await submit();
  assert.equal(elements.status.textContent, 'White to move.'); assert(elements['clock-white'].classes.has('active'));
  assert.equal(elements['white-time'].textContent, '2:00'); assert.equal(elements['black-time'].textContent, '2:00');
  assert(elements['history-table'].hidden); assert(elements['engine-stats'].hidden);
  assert.equal(elements['engine-status'].textContent, 'The computer is off in two-player mode.');
  clickSquare(12); assert(elements.board.children.find(e => e.dataset.square === 28).classes.has('target'));
  clickSquare(28); await tick(); assert(requests.at(-1).url.includes('from=12&to=28&promotion=0'));
  assert.equal(elements.status.textContent, 'Black to move.'); assert(elements['clock-black'].classes.has('active'));
  assert(!elements['history-table'].hidden); assert.equal(elements.history.children.length, 1);
  vm.runInContext('state.receivedAt=performance.now()-2200;renderClocks()', context);
  assert.equal(elements['black-time'].textContent, '1:58');
  elements.flip.onclick(); assert.equal(elements.board.children[0].dataset.square, 7);
  vm.runInContext("state={...state,white:true,board:Array(64).fill(0),result:null,thinking:false,legal:[[48,56,5],[48,56,4],[48,56,3],[48,56,2]]};state.board[48]=1;flipped=false;selected=null;render()", context);
  clickSquare(48); clickSquare(56); assert.equal(elements['promotion-choices'].children.length, 4);
  assert(elements['promotion-dialog'].open); elements['promotion-choices'].children[3].onclick(); await tick();
  assert(requests.at(-1).url.includes('promotion=2'));
  // Custom controls remain available in both modes, including fractional minutes.
  elements['menu-button'].onclick(); mode = 'one'; color = 'black'; elements['time-control'].value = 'custom';
  elements['time-control'].listeners.change(); assert(!elements['custom-time'].hidden); assert(!elements['base-minutes'].disabled);
  elements['base-minutes'].value = '1.5'; elements['increment-seconds'].value = '3'; await submit();
  assert(requests.at(-1).url.includes('base_seconds=90&increment_seconds=3'));
  assert.equal(elements.board.children[0].dataset.square, 7);
  assert.equal(elements['game-mode'].textContent, 'You: Black · 1.5 min + 3 sec');
  assert.equal(elements.status.textContent, 'Computer thinking…'); assert(!elements['engine-info'].open);
  elements['engine-info'].open = true;
  vm.runInContext('state={...state,thinking:false,white:false,stats:{depth:6,nodes:12345,hits:123,millis:2500,budget_millis:2600}};render()', context);
  assert(elements['engine-info'].open); assert(!elements['engine-stats'].hidden);
  assert.equal(Number(elements['engine-depth'].textContent), 6);
  assert.equal(elements['engine-nodes'].textContent, (12345).toLocaleString());
  assert.equal(elements['engine-budget'].textContent, '2.60 s'); assert.equal(elements['engine-time'].textContent, '2.50 s');
  // Clock-only responses must resynchronize even without a board revision.
  server.thinking = false; server.white = false; server.clocks.active_white = false; server.clocks.black_ms = 5000;
  await vm.runInContext('poll()', context);
  assert.equal(elements['black-time'].textContent, '5.0'); assert(elements['clock-black'].classes.has('low-time'));
  const before = requests.length;
  vm.runInContext('state.clocks.black_ms=0;state.legal=[[52,44,0]];render()', context);
  clickSquare(52); clickSquare(44); await tick(); assert.equal(requests.length, before);
  // Unlimited disables game clocks and offers all five thinking presets.
  elements['menu-button'].onclick(); mode = 'one'; color = 'white';
  elements['time-control'].value = 'unlimited'; elements['time-control'].listeners.change();
  assert(!elements['unlimited-thinking'].hidden); assert(!elements['thinking-time'].disabled);
  for (const seconds of ['1','2','3','4','5']) {
    elements['thinking-time'].value = seconds; elements['thinking-time'].listeners.change();
    assert(elements['custom-thinking'].hidden); assert(elements['thinking-seconds'].disabled);
    await submit(); const params = new URL(requests.at(-1).url, 'http://localhost').searchParams;
    assert.equal(params.get('thinking_seconds'), seconds); assert.equal(params.get('unlimited'), 'true');
    assert(elements['game-clocks'].hidden); assert(elements['game-mode'].textContent.includes(seconds + 's per move'));
    elements['menu-button'].onclick();
  }
  elements['thinking-time'].value = 'custom'; elements['thinking-time'].listeners.change();
  assert(!elements['custom-thinking'].hidden); assert(!elements['thinking-seconds'].disabled);
  elements['thinking-seconds'].value = '13.75'; await submit();
  assert(requests.at(-1).url.includes('thinking_seconds=13.75')); assert(elements['game-clocks'].hidden);
  clickSquare(12); clickSquare(28); await tick();
  assert(requests.at(-1).url.includes('/api/move?')); // Zero-valued disabled clocks must not block play.
  elements['menu-button'].onclick(); mode = 'two'; vm.runInContext('changeMode()', context);
  assert(elements['unlimited-thinking'].hidden); assert(elements['thinking-seconds'].disabled);
  await submit(); assert.equal(elements['game-mode'].textContent, 'Two players · Unlimited');
  assert(elements['game-clocks'].hidden);
  elements['menu-button'].onclick(); elements['time-control'].value = '5'; elements['time-control'].listeners.change();
  assert(elements['unlimited-thinking'].hidden); await submit(); assert(!elements['game-clocks'].hidden);
  // Sound follows accepted game events, including computer moves from polling.
  audioNotes.length = 0;
  clickSquare(12); await tick(); assert.equal(audioNotes.length, 0);
  clickSquare(28); await tick(); assert.equal(audioNotes.length, 1);
  assert.equal(audioNotes[0].frequency, 640); assert(audioNotes[0].endFrequency < audioNotes[0].frequency);
  audioNotes.length = 0;
  await vm.runInContext('poll()', context); elements.flip.onclick(); await tick(); assert.equal(audioNotes.length, 0);
  server.notation.push('Qh4+'); server.revision++; server.check = true;
  await vm.runInContext('poll()', context); await tick();
  assert.deepEqual(audioNotes.map(n => n.frequency), [660, 880]);
  audioNotes.length = 0;
  server.notation.push('Qh4#'); server.revision++; server.result = 'Black wins by checkmate'; server.clocks.running = false;
  await vm.runInContext('poll()', context); await tick();
  assert.deepEqual(audioNotes.map(n => n.frequency), [660, 523.25, 392]); // End cue takes priority over check and move.
  audioNotes.length = 0;
  await vm.runInContext('poll()', context); await elements.undo.onclick(); await tick();
  assert.equal(audioNotes.length, 0); // No replay on duplicate responses or undo.
  elements['sound-toggle'].onclick(); assert.equal(elements['sound-toggle'].attrs['aria-pressed'], 'false');
  assert.equal(storedSounds.get('chess.sounds'), 'off');
  await elements['confirm-resign'].onclick(); await tick(); assert.equal(audioNotes.length, 0);
  elements['sound-toggle'].onclick(); await tick(); assert.equal(audioNotes.length, 0);
  assert.equal(elements['sound-toggle'].attrs['aria-pressed'], 'true');
  await vm.runInContext('poll()', context); await tick(); assert.equal(audioNotes.length, 0);
  elements['menu-button'].onclick(); await submit();
  audioNotes.length = 0; rejectNextMove = true;
  clickSquare(12); clickSquare(28); await tick(); assert.equal(audioNotes.length, 0);
  assert(!elements.error.hidden); elements['dismiss-error'].onclick();
  clickSquare(12); clickSquare(28); await tick(); assert.equal(audioNotes.length, 1);
  audioNotes.length = 0;
  elements['menu-button'].onclick(); await elements.resume.onclick(); await tick(); assert.equal(audioNotes.length, 0);
  // Timeout has no new move, but still gets the game-end sound once.
  server.result = 'Black wins on time'; server.revision++; server.clocks.running = false;
  await vm.runInContext('poll()', context); await tick(); assert.equal(audioNotes.length, 3);
  // Browsers without audio support must keep the game playable.
  audioNotes.length = 0;
  vm.runInContext('cancelSounds();audioContext=null;window.AudioContext=undefined', context);
  elements['menu-button'].onclick(); await submit(); clickSquare(12); clickSquare(28); await tick();
  assert.equal(elements.status.textContent, 'Black to move.'); assert.equal(audioNotes.length, 0);
  console.log('UI harness passed: timed and unlimited controls, clocks, moves, promotion, engine diagnostics, move/check/end sounds, mute, duplicate suppression, and audio fallback.');
})().catch(e => {console.error(e); process.exitCode = 1;});
