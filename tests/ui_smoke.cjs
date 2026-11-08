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
  remove() { documentBody.children = documentBody.children.filter(e => e !== this); }
  replaceChildren(...args) { this.children = []; args.forEach(e => this.append(e)); }
  setAttribute(k, v) { this.attrs[k] = String(v); }
  focus() { focused = this; }
  contains(e) { return this.children.includes(e); }
  querySelector(s) { const m = s.match(/data-square="(\d+)"/); return m ? this.children.find(e => String(e.dataset.square) === m[1]) : null; }
  addEventListener(name, fn) { this.listeners[name] = fn; }
  getBoundingClientRect() {
    const index = elements.board.children.indexOf(this);
    return {left: index % 8 * 80, top: Math.floor(index / 8) * 80, width: 80, height: 80};
  }
  setPointerCapture(id) { this.capturedPointer = id; }
  hasPointerCapture(id) { return this.capturedPointer === id; }
  releasePointerCapture() { this.capturedPointer = null; }
  closest() { return this; }
  showModal() { this.open = true; }
  close() { this.open = false; this.listeners.close?.(); }
}
const elements = {};
const documentBody = new Element(), documentListeners = {}, windowListeners = {};
for (const m of html.matchAll(/<[^>]*\bid="([^"]+)"[^>]*>/g)) {
  const e = elements[m[1]] = new Element(); e.hidden = /\bhidden\b/.test(m[0]);
  e.disabled = /\bdisabled\b/.test(m[0]); e.value = m[0].match(/\bvalue="([^"]+)"/)?.[1] || '';
}
elements['time-control'].value = '5';
elements['thinking-time'].value = '3';
elements['two-player-mode'].value = 'same';
let mode = 'two', color = 'white', server, requests = [], rejectNextMove = false, afterNextMove = null, pollDelay, offline = false;
let confirmationTimeout = null, copiedFen = null, clipboardFails = false;
const initial = [4,2,3,5,6,3,2,4,...Array(8).fill(1),...Array(32).fill(0),...Array(8).fill(-1),-4,-2,-3,-5,-6,-3,-2,-4];
const fresh = (human = null, base = 300, increment = 0) => ({
  board: [...initial], started_at: new Date(2026, 10, 8, 12).getTime() / 1000, white: true, human, thinking_seconds: 3, revision: 1, thinking: false, pondering: human === true, check: false, result: null,
  legal: [[12,20,0],[12,28,0]], notation: [], last: null, stats: null, analysis: null,
  clocks: {enabled: true, white_ms: base * 1000, black_ms: base * 1000, running: true, active_white: true, base_seconds: base, increment_seconds: increment}
});
server = fresh(); server.clocks.running = false;
const context = vm.createContext({
  console, URLSearchParams, Set, AbortSignal, performance,
  navigator: {clipboard: {writeText: async text => { if (clipboardFails) throw new Error('denied'); copiedFen = text; }}},
  clearTimeout: id => { if (id === 5000) confirmationTimeout = null; },
  setTimeout: (fn, delay) => { if (delay === 5000) { confirmationTimeout = fn; return 5000; } pollDelay = delay; return 0; }, setInterval: () => 0,
  window: {scrollTo() {}, addEventListener: (name, fn) => { windowListeners[name] = fn; }, AudioContext: AudioContextMock, localStorage: {getItem: key => storedSounds.get(key) ?? null, setItem: (key, value) => storedSounds.set(key, value)}},
  FormData: class {get(k) {return {mode, color, 'two-player-mode': elements['two-player-mode'].value, 'time-control': elements['time-control'].value, minutes: elements['base-minutes'].value, increment: elements['increment-seconds'].value, 'thinking-time': elements['thinking-time'].value, 'thinking-seconds': elements['thinking-seconds'].value}[k];}},
  document: {hidden: false, body: documentBody, addEventListener: (name, fn) => { documentListeners[name] = fn; }, elementFromPoint: (x, y) => x < 0 || y < 0 || x >= 640 || y >= 640 ? null : elements.board.children[Math.floor(y / 80) * 8 + Math.floor(x / 80)], getElementById: id => elements[id], createElement: () => new Element(), createDocumentFragment: () => new Element(true), get activeElement() {return focused;}, querySelector: () => ({value: mode}), querySelectorAll: () => [new Element(), new Element()]},
  fetch: async (url, options = {}) => {
    requests.push({url, options}); const u = new URL(url, 'http://localhost');
    if (offline) throw new Error('Network error');
    if (u.pathname === '/api/move' && rejectNextMove) {
      rejectNextMove = false; return {ok: false, json: async () => ({error: 'That move is not legal.'})};
    }
    if (u.pathname === '/api/new') {
      server = fresh(u.searchParams.get('mode') === 'two' ? null : u.searchParams.get('color') === 'white', Number(u.searchParams.get('base_seconds')), Number(u.searchParams.get('increment_seconds')));
      if (u.searchParams.get('unlimited') === 'true') {
        server.thinking_seconds = Number(u.searchParams.get('thinking_seconds') || 3);
        server.clocks.enabled = false; server.clocks.running = false; server.clocks.active_white = null;
      }
      server.ai_vs_ai = u.searchParams.get('two_player_mode') === 'ai';
      server.thinking = server.ai_vs_ai || server.human === false;
      if (server.ai_vs_ai) server.pondering = false;
    }
    if (u.pathname === '/api/move') {
      const from = Number(u.searchParams.get('from')), to = Number(u.searchParams.get('to'));
      const pc = server.board[from], promotion = Number(u.searchParams.get('promotion'));
      if (Math.abs(pc) === 1 && from % 8 !== to % 8 && !server.board[to]) {
        const captured = to + (pc > 0 ? -8 : 8);
        if (server.board[captured] === -pc) server.board[captured] = 0;
      }
      if (Math.abs(pc) === 6 && Math.abs(to - from) === 2) {
        const rookFrom = from + (to > from ? 3 : -4), rookTo = from + (to > from ? 1 : -1);
        server.board[rookTo] = server.board[rookFrom]; server.board[rookFrom] = 0;
      }
      server.board[to] = promotion ? (pc > 0 ? promotion : -promotion) : pc;
      server.board[from] = 0;
      server.clocks[server.white ? 'white_ms' : 'black_ms'] += server.clocks.increment_seconds * 1000;
      server = {...server, revision: server.revision + 1, white: !server.white, notation: [...server.notation, 'e4'], last: [from,to]};
      server.thinking = server.human !== null && server.white !== server.human;
      server.pondering = server.human !== null && !server.thinking;
      if (server.clocks.enabled) server.clocks.active_white = server.white;
      if (afterNextMove) { const callback = afterNextMove; afterNextMove = null; callback(); }
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
const square = s => elements.board.children.find(e => e.dataset.square === s);
const clickSquare = (s, detail = 1) => elements.board.listeners.click({target: square(s), detail, preventDefault() {}});
const pointer = (s, overrides = {}) => {
  const target = square(s), rect = target?.getBoundingClientRect();
  return {target, pointerId: 1, pointerType: 'mouse', isPrimary: true, button: 0,
    clientX: rect ? rect.left + 40 : -20, clientY: rect ? rect.top + 40 : -20, preventDefault() {}, ...overrides};
};
const startDrag = (from, to, overrides = {}) => {
  elements.board.listeners.pointerdown(pointer(from, overrides));
  documentListeners.pointermove(pointer(to, overrides));
};
const drop = (to, overrides = {}) => {
  documentListeners.pointerup(pointer(to, overrides));
  if (to !== null) clickSquare(to); // Browser compatibility click must not move twice or change selection.
};
const openMenu = () => { elements['menu-button'].onclick(); if (elements.menu.hidden) elements['menu-button'].onclick(); };
const moveRequests = () => requests.filter(r => r.url.startsWith('/api/move?'));
(async () => {
  await tick(); assert(!elements.menu.hidden); assert(elements.game.hidden);
  await elements['copy-pgn'].onclick(); assert.match(copiedFen, /\[Result "\*"\]\n\n\*\n$/);
  assert.equal(copiedFen, '[Event "5 min"]\n[Site "github.com/bkthomps/chess-ai"]\n[Date "2026.11.08"]\n[White "Player"]\n[Black "Player"]\n[Result "*"]\n\n*\n');
  assert(elements['copy-pgn'].classes.has('copied'));
  assert.match(vm.runInContext("gamePgn({...state,human:false,ai_vs_ai:false})", context), /\[White "Computer"\]\n\[Black "Player"\]/);
  assert.match(vm.runInContext("gamePgn({...state,human:null,ai_vs_ai:true,thinking_seconds:2.5,clocks:{enabled:false}})", context), /\[Event "Unlimited, 2\.5 sec computer thinking time per move"\][\s\S]*\[White "Computer"\]\n\[Black "Computer"\]/);
  assert.match(vm.runInContext("gamePgn({...state,human:true,thinking_seconds:1,clocks:{enabled:false}})", context), /\[Event "Unlimited, 1 sec computer thinking time per move"\]/);
  assert.match(vm.runInContext("gamePgn({...state,human:null,ai_vs_ai:false,clocks:{enabled:false}})", context), /\[Event "Unlimited"\]/);
  assert.match(vm.runInContext("gamePgn({...state,clocks:{enabled:true,base_seconds:180,increment_seconds:2},notation:['e4','e5','Nf3'],result:'Black wins by resignation'})", context), /\[Event "3 min \+ 2 sec"\][\s\S]*\[Result "0-1"\]\n\n1\. e4 e5 2\. Nf3 0-1\n$/);
  assert.match(vm.runInContext("gamePgn({...state,notation:['d4'],result:'Draw by stalemate'})", context), /1\. d4 1\/2-1\/2\n$/);
  clipboardFails = true; await elements['copy-pgn'].onclick();
  assert(!elements['copy-pgn'].classes.has('copied'));
  assert.match(elements['error-text'].textContent, /Could not copy/); clipboardFails = false;
  elements['dismiss-error'].onclick();
  assert.equal(audioNotes.length, 31); // The longer menu theme begins once audio is available.
  assert.equal(vm.runInContext('musicTheme', context), 'menu');
  assert.equal(vm.runInContext('musicActive', context), true);
  offline = true; await vm.runInContext('poll()', context);
  assert(documentBody.classes.has('disconnected')); assert(!elements.disconnected.hidden);
  await tick(); assert.equal(vm.runInContext('musicTheme', context), 'disconnected');
  assert.equal(vm.runInContext('musicActive', context), true);
  assert(elements['disconnected-king'].innerHTML.includes('stroke="#b33a3a"'));
  offline = false; await vm.runInContext('poll()', context);
  assert(!documentBody.classes.has('disconnected')); assert(elements.disconnected.hidden);
  await tick(); assert.equal(vm.runInContext('musicTheme', context), 'menu');
  await submit(); assert.equal(vm.runInContext('musicActive', context), false); offline = true; await vm.runInContext('poll()', context); await tick();
  assert.equal(vm.runInContext('musicActive', context), true);
  assert(documentBody.classes.has('disconnected')); assert(!elements.disconnected.hidden);
  offline = false; await vm.runInContext('poll()', context);
  assert(!documentBody.classes.has('disconnected')); assert(elements.game.hidden);
  assert(!elements.menu.hidden); assert.equal(elements.board.children.length, 0);
  assert.equal(vm.runInContext('state', context), null);
  for (const [human, white, fill] of [[true, false, '#fffaf0'], [false, true, '#263e33'], [null, true, '#fffaf0'], [null, false, '#263e33']]) {
    await submit(); vm.runInContext(`state.human=${human};state.white=${white}`, context);
    offline = true; await vm.runInContext('poll()', context);
    assert(elements['disconnected-king'].innerHTML.includes(`fill="${fill}"`));
    offline = false; await vm.runInContext('poll()', context);
  }
  vm.runInContext('changeMode()', context); assert(elements['solo-options'].hidden);
  // Every preset must reach the server in seconds, including both increments.
  for (const [preset, base, increment] of [['1',60,0], ['2+1',120,1], ['5',300,0], ['10+5',600,5]]) {
    elements['time-control'].value = preset; elements['time-control'].listeners.change();
    assert(elements['custom-time'].hidden); assert(elements['base-minutes'].disabled);
    await submit(); const params = new URL(requests.at(-1).url, 'http://localhost').searchParams;
    assert.equal(Number(params.get('base_seconds')), base); assert.equal(Number(params.get('increment_seconds')), increment);
    assert.equal(elements.board.children.length, 64);
    openMenu();
  }
  elements['time-control'].value = '2+1'; await submit();
  elements['menu-button'].onclick();
  assert(!elements.game.hidden); assert.equal(elements['menu-button'].textContent, 'Confirm');
  assert(elements['menu-button'].classes.has('confirm'));
  documentListeners.click({target: elements.board});
  assert.equal(elements['menu-button'].textContent, 'New game'); assert.equal(confirmationTimeout, null);
  elements['menu-button'].onclick(); assert(confirmationTimeout);
  confirmationTimeout(); assert.equal(elements['menu-button'].textContent, 'New game');
  assert(!elements['menu-button'].classes.has('confirm'));
  elements['menu-button'].onclick();
  elements['menu-button'].onclick(); assert(!elements.menu.hidden);
  await submit(); assert.equal(elements['menu-button'].textContent, 'New game');
  assert(!elements['menu-button'].classes.has('confirm'));
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
  openMenu(); mode = 'one'; color = 'black'; elements['time-control'].value = 'custom';
  elements['time-control'].listeners.change(); assert(!elements['custom-time'].hidden); assert(!elements['base-minutes'].disabled);
  elements['base-minutes'].value = '1.5'; elements['increment-seconds'].value = '3'; await submit();
  assert(requests.at(-1).url.includes('base_seconds=90&increment_seconds=3'));
  assert.equal(elements.board.children[0].dataset.square, 7);
  assert.equal(elements['game-mode'].textContent, 'You: Black · 1.5 min + 3 sec');
  assert.equal(elements.status.textContent, 'Computer thinking…'); assert(!elements['engine-info'].open);
  elements['engine-info'].open = true;
  vm.runInContext('state={...state,thinking:false,pondering:true,white:false,stats:{depth:6,nodes:12345,hits:123,millis:2500,budget_millis:2600,threads:6}};render()', context);
  assert(elements['engine-info'].open); assert(!elements['engine-stats'].hidden);
  assert.equal(elements.status.textContent, 'Black to move.');
  assert.equal(elements['engine-status'].textContent, 'Pondering possible replies.');
  assert(vm.runInContext('canPlay()', context));
  assert.equal(Number(elements['engine-depth'].textContent), 6);
  assert.equal(Number(elements['engine-threads'].textContent), 6);
  assert.equal(elements['engine-nodes'].textContent, (12345).toLocaleString());
  assert.equal(elements['engine-time'].textContent, '2.50 s / 2.60 s');
  // Live updates arrive without a board revision, including Unlimited pondering.
  server.thinking = false; server.pondering = true; server.white = false;
  server.clocks.enabled = false; server.clocks.running = false;
  server.analysis = {revision: server.revision, active: true, pondering: true, depth: 4, searching_depth: 5, nodes: 1000, hits: 12, millis: 250, budget_millis: null, threads:3, cache_bytes:65463040, cache_limit:65494144};
  await vm.runInContext('poll()', context);
  vm.runInContext('state.legal=[[52,44,0]];selected=52;renderBoard()', context);
  const selectedButton = elements.board.children.find(e => e.dataset.square === 52);
  const soundsBeforeLive = audioNotes.length;
  server.analysis = {...server.analysis, nodes: 5000, hits: 40, millis: 500};
  await vm.runInContext('poll()', context); await tick();
  assert.equal(pollDelay, 150);
  assert.equal(elements['engine-nodes'].textContent, (5000).toLocaleString());
  assert.equal(elements['engine-hits'].textContent, (40).toLocaleString());
  assert.equal(elements['engine-time'].textContent, '0.50 s / Your turn');
  assert.equal(Number(elements['engine-threads'].textContent), 3);
  assert.equal(elements['engine-cache'].textContent, '65.5 / 65.5 MB');
  assert.equal(vm.runInContext('selected', context), 52);
  assert.equal(elements.board.children.find(e => e.dataset.square === 52), selectedButton);
  assert.equal(audioNotes.length, soundsBeforeLive);
  server.thinking = true; server.pondering = false;
  server.analysis = {...server.analysis, pondering: false, nodes: 0, hits: 0, millis: 0, budget_millis: 2600};
  await vm.runInContext('poll()', context);
  server.analysis = {...server.analysis, nodes: 2000, millis: 250};
  await vm.runInContext('poll()', context);
  assert.equal(elements['engine-status'].textContent, 'Thinking… Live search.');
  assert.equal(elements['engine-nodes'].textContent, (2000).toLocaleString());
  assert.equal(elements['engine-time'].textContent, '0.25 s / 2.60 s');
  // Clock-only responses must resynchronize even without a board revision.
  server.thinking = false; server.white = false; server.clocks.enabled = true; server.clocks.running = true;
  server.clocks.active_white = false; server.clocks.black_ms = 5000;
  server.analysis = {...server.analysis, active: false, searching_depth: 0};
  await vm.runInContext('poll()', context);
  assert.equal(elements['black-time'].textContent, '5.0'); assert(elements['clock-black'].classes.has('low-time'));
  const before = requests.length;
  vm.runInContext('state.clocks.black_ms=0;state.legal=[[52,44,0]];render()', context);
  clickSquare(52); clickSquare(44); await tick(); assert.equal(requests.length, before);
  // Unlimited disables game clocks and offers all five thinking presets.
  openMenu(); mode = 'one'; color = 'white';
  elements['time-control'].value = 'unlimited'; elements['time-control'].listeners.change();
  assert(!elements['unlimited-thinking'].hidden); assert(!elements['thinking-time'].disabled);
  for (const seconds of ['1','2','3','4','5']) {
    elements['thinking-time'].value = seconds; elements['thinking-time'].listeners.change();
    assert(elements['custom-thinking'].hidden); assert(elements['thinking-seconds'].disabled);
    await submit(); const params = new URL(requests.at(-1).url, 'http://localhost').searchParams;
    assert.equal(params.get('thinking_seconds'), seconds); assert.equal(params.get('unlimited'), 'true');
    assert(elements['game-clocks'].hidden); assert(elements['game-mode'].textContent.includes(seconds + 's per move'));
    openMenu();
  }
  elements['thinking-time'].value = 'custom'; elements['thinking-time'].listeners.change();
  assert(!elements['custom-thinking'].hidden); assert(!elements['thinking-seconds'].disabled);
  elements['thinking-seconds'].value = '13.75'; await submit();
  assert(requests.at(-1).url.includes('thinking_seconds=13.75')); assert(elements['game-clocks'].hidden);
  clickSquare(12); clickSquare(28); await tick();
  assert(requests.at(-1).url.includes('/api/move?')); // Zero-valued disabled clocks must not block play.
  openMenu(); mode = 'two'; vm.runInContext('changeMode()', context);
  assert(elements['unlimited-thinking'].hidden); assert(elements['thinking-seconds'].disabled);
  await submit(); assert.equal(elements['game-mode'].textContent, 'Two players · Unlimited');
  assert(elements['game-clocks'].hidden);
  openMenu(); elements['time-control'].value = '5'; elements['time-control'].listeners.change();
  assert(elements['unlimited-thinking'].hidden); await submit(); assert(!elements['game-clocks'].hidden);
  // AI spectators cannot move or queue premoves, including between searches.
  openMenu(); elements['two-player-mode'].value = 'ai';
  elements['time-control'].value = 'unlimited'; elements['two-player-mode'].listeners.change();
  assert(!elements['two-player-options'].hidden); assert(!elements['unlimited-thinking'].hidden);
  await submit();
  assert.equal(new URL(requests.at(-1).url, 'http://localhost').searchParams.get('two_player_mode'), 'ai');
  assert.equal(elements['game-mode'].textContent, 'AI vs. AI · Unlimited · 13.75s per move');
  assert.equal(elements.status.textContent, 'White computer thinking…');
  vm.runInContext('state.clocks.enabled=true;renderClocks()', context);
  assert.equal(elements['white-clock-label'].textContent, 'White · Computer');
  assert.equal(elements['black-clock-label'].textContent, 'Black · Computer');
  assert(elements.resign.disabled);
  vm.runInContext('state.thinking=false;render()', context);
  assert.equal(vm.runInContext('canPlay() || canPremove()', context), false);
  server.thinking = true;
  server.analysis = {active:true, pondering:false, depth:2, searching_depth:3, nodes:100, hits:4, millis:10, budget_millis:13750, threads:1, cache_bytes:1024, cache_limit:2048};
  await vm.runInContext('poll()', context);
  assert(!elements['engine-stats'].hidden);
  openMenu(); mode = 'one'; vm.runInContext('changeMode()', context);
  assert(elements['two-player-options'].hidden); assert(elements['two-player-mode'].disabled);
  mode = 'two'; elements['two-player-mode'].value = 'same'; vm.runInContext('changeMode()', context);
  elements['time-control'].value = '5'; elements['time-control'].listeners.change(); await submit();
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
  elements['menu-button'].onclick(); assert.equal(elements['menu-button'].textContent, 'Confirm');
  await vm.runInContext('poll()', context); await tick();
  assert.deepEqual(audioNotes.map(n => n.frequency), [660, 523.25, 392]); // End cue takes priority over check and move.
  assert.equal(elements['menu-button'].textContent, 'New game');
  elements['menu-button'].onclick(); assert(!elements.menu.hidden); await submit();
  audioNotes.length = 0;
  await vm.runInContext('poll()', context); await elements.undo.onclick(); await tick();
  assert.equal(audioNotes.length, 0); // No replay on duplicate responses or undo.
  openMenu(); await submit();
  audioNotes.length = 0; rejectNextMove = true;
  clickSquare(12); clickSquare(28); await tick(); assert.equal(audioNotes.length, 0);
  assert(!elements.error.hidden); elements['dismiss-error'].onclick();
  clickSquare(12); clickSquare(28); await tick(); assert.equal(audioNotes.length, 1);
  audioNotes.length = 0;
  openMenu(); await tick(); assert.equal(audioNotes.length, 31);
  await submit(); await tick(); assert.equal(vm.runInContext('musicActive', context), false);
  audioNotes.length = 0;
  // Timeout has no new move, but still gets the game-end sound once.
  server.result = 'Black wins on time'; server.revision++; server.clocks.running = false;
  await vm.runInContext('poll()', context); await tick(); assert.equal(audioNotes.length, 3);
  // Browsers without audio support must keep the game playable.
  audioNotes.length = 0;
  vm.runInContext('cancelSounds();audioContext=null;window.AudioContext=undefined', context);
  openMenu(); await submit(); clickSquare(12); clickSquare(28); await tick();
  assert.equal(elements.status.textContent, 'Black to move.'); assert.equal(audioNotes.length, 0);
  // Pointer gestures share click-to-move's legality and promotion flow.
  mode = 'two'; elements['time-control'].value = 'unlimited';
  const resetBoard = async () => { openMenu(); await submit(); };
  await resetBoard();
  const movesBeforeDrag = moveRequests().length;
  startDrag(12, 28);
  assert(square(12).classes.has('drag-source')); assert(square(28).classes.has('drag-over'));
  assert(elements.board.hasPointerCapture(1)); assert.equal(documentBody.children.length, 1);
  drop(28); await tick();
  assert.equal(moveRequests().length, movesBeforeDrag + 1);
  assert(moveRequests().at(-1).url.includes('from=12&to=28&promotion=0'));
  assert.equal(documentBody.children.length, 0); assert(!elements.board.hasPointerCapture(1));
  // Short presses, including minor pointer movement, still select and click to move.
  await resetBoard();
  elements.board.listeners.pointerdown(pointer(12));
  documentListeners.pointermove(pointer(12, {clientX:pointer(12).clientX + 2}));
  documentListeners.pointerup(pointer(12)); clickSquare(12);
  assert(square(12).classes.has('selected')); clickSquare(28); await tick();
  assert.equal(elements.status.textContent, 'Black to move.');
  // Illegal and outside drops retain the piece and legal hints. Keyboard clicks remain usable.
  await resetBoard();
  const beforeInvalid = moveRequests().length;
  startDrag(12, 36); drop(36); await tick();
  assert.equal(moveRequests().length, beforeInvalid); assert(square(12).classes.has('selected'));
  startDrag(12, null); drop(null); await tick();
  assert.equal(moveRequests().length, beforeInvalid); assert.equal(documentBody.children.length, 0);
  clickSquare(28, 0); await tick(); assert.equal(moveRequests().length, beforeInvalid + 1);
  // Board orientation does not change square identity; touch can capture a piece.
  await resetBoard(); elements.flip.onclick();
  vm.runInContext('state.board[28]=-1', context); vm.runInContext('renderBoard()', context);
  startDrag(12, 28, {pointerType:'touch'}); drop(28, {pointerType:'touch'}); await tick();
  assert(moveRequests().at(-1).url.includes('from=12&to=28&promotion=0'));
  await resetBoard();
  vm.runInContext('state={...state,board:Array(64).fill(0),legal:[[48,56,5],[48,56,4],[48,56,3],[48,56,2]]};state.board[48]=1;renderBoard()', context);
  const beforePromotion = moveRequests().length;
  startDrag(48, 56); drop(56);
  assert(elements['promotion-dialog'].open); assert.equal(moveRequests().length, beforePromotion);
  elements['promotion-choices'].children[2].onclick(); await tick();
  assert(moveRequests().at(-1).url.includes('from=48&to=56&promotion=3'));
  // Cancellation and board changes remove the floating piece without sending a move.
  await resetBoard();
  const beforeCancel = moveRequests().length;
  for (const cancel of [
    () => documentListeners.pointercancel(pointer(28)),
    () => elements.board.listeners.lostpointercapture(pointer(28, {target:elements.board})),
    () => windowListeners.blur(),
    () => documentListeners.keydown({key:'Escape'}),
    () => elements.flip.onclick(),
    () => vm.runInContext('state.revision++;selected=null;render()', context),
    () => openMenu()
  ]) {
    startDrag(12, 28); assert.equal(documentBody.children.length, 1);
    cancel(); drop(28); await tick();
    assert.equal(documentBody.children.length, 0); assert.equal(moveRequests().length, beforeCancel);
    if (elements.game.hidden) await submit();
  }
  // Opponent pieces, non-primary pointers, and unavailable turns never start a drag.
  startDrag(52, 36); assert.equal(documentBody.children.length, 0); drop(36);
  startDrag(12, 28, {isPrimary:false}); assert.equal(documentBody.children.length, 0); drop(28);
  for (const blocked of ['busy=true', 'state.thinking=true', "state.result='Draw'", 'state.human=false', 'state.clocks.enabled=true;state.clocks.white_ms=0']) {
    await resetBoard(); vm.runInContext(blocked, context);
    startDrag(12, 28); drop(28); await tick(); assert.equal(documentBody.children.length, 0);
    assert.equal(moveRequests().length, beforeCancel); vm.runInContext('busy=false', context);
  }
  // Premoves are local intent until the next computer reply supplies legal moves.
  const waitForComputer = async (human = true, edit = () => {}) => {
    mode = 'one'; color = human ? 'white' : 'black'; await resetBoard();
    server.thinking = true; server.pondering = false; server.white = !human;
    server.revision++;
    if (human) {
      server.notation = ['e4']; server.board[12] = 0; server.board[28] = 1;
    }
    server.legal = human ? [[52,36,0]] : [[12,28,0]];
    edit(server);
    await vm.runInContext('poll()', context);
  };
  const computerReply = async (legal, edit = () => {}, reply = server.human ? [52,36] : [12,28]) => {
    const [from, to] = reply;
    server.revision++; server.notation.push(server.human ? 'e5' : 'e4');
    server.board[to] = server.board[from]; server.board[from] = 0; server.last = [from,to];
    server.white = server.human; server.thinking = false; server.pondering = true; server.legal = legal;
    edit(server);
    await vm.runInContext('poll()', context); await tick();
  };
  await waitForComputer();
  const beforePremove = moveRequests().length;
  clickSquare(6); assert(square(21).classes.has('premove-target'));
  assert(square(21).attrs['aria-label'].includes('possible premove'));
  clickSquare(21); await tick();
  assert.equal(moveRequests().length, beforePremove); assert(square(6).classes.has('premoved'));
  assert(square(21).classes.has('premoved')); assert(!elements['cancel-premove'].hidden);
  assert.equal(elements['premove-status'].textContent, '1 premove queued · Board preview');
  assert.equal(elements['premove-queue'].children[0].textContent, 'g1 → f3');
  const queuedButton = square(21);
  await vm.runInContext('poll()', context);
  assert.equal(square(21), queuedButton); assert.equal(moveRequests().length, beforePremove);
  const replyRevision = server.revision + 1;
  await computerReply([[6,21,0]]);
  assert.equal(moveRequests().length, beforePremove + 1);
  assert(moveRequests().at(-1).url.includes(`from=6&to=21&promotion=0&revision=${replyRevision}`));
  assert(elements['cancel-premove'].hidden);
  await vm.runInContext('poll()', context); await tick();
  assert.equal(moveRequests().length, beforePremove + 1); // Never replay after duplicate responses.
  // Black can queue touch and keyboard moves on the flipped board, in order.
  await waitForComputer(false);
  assert.equal(elements.board.children[0].dataset.square, 7);
  const beforeBlack = moveRequests().length;
  startDrag(52, 36, {pointerType:'touch'}); drop(36, {pointerType:'touch'});
  assert.equal(elements['premove-queue'].children[0].textContent, 'e7 → e5');
  clickSquare(62, 0); clickSquare(45, 0);
  assert(square(52).classes.has('premoved')); assert(square(45).classes.has('premoved'));
  assert.equal(elements['premove-queue'].children.length, 2);
  await computerReply([[52,36,0],[62,45,0]]);
  assert.equal(moveRequests().length, beforeBlack + 1);
  assert(moveRequests().at(-1).url.includes('from=52&to=36&promotion=0'));
  assert.equal(elements['premove-queue'].children.length, 1);
  await computerReply([[62,45,0]], () => {}, [6,21]);
  assert.equal(moveRequests().length, beforeBlack + 2);
  assert(moveRequests().at(-1).url.includes('from=62&to=45&promotion=0'));
  await waitForComputer(false, s => { s.board[8]=-1; s.board[0]=0; });
  const beforeBlackPromotion = moveRequests().length;
  clickSquare(8); clickSquare(0); assert(elements['promotion-dialog'].open);
  assert(elements['promotion-choices'].children[0].innerHTML.includes('fill="#263e33"'));
  elements['promotion-choices'].children[1].onclick();
  await computerReply([[8,0,5],[8,0,4]]);
  assert.equal(moveRequests().length, beforeBlackPromotion + 1);
  assert(moveRequests().at(-1).url.includes('from=8&to=0&promotion=4'));
  await waitForComputer(); const beforeImpossible = moveRequests().length;
  clickSquare(6); clickSquare(22); // A knight cannot move straight forward.
  assert(elements['cancel-premove'].hidden); await computerReply([[6,21,0]]);
  assert.equal(moveRequests().length, beforeImpossible);
  // Conditional pawn captures, en passant, and castling use the fresh legal list.
  for (const [from, to, edit] of [
    [11,20, () => {}], // Empty diagonal now; the opponent can move a piece there.
    [28,37, s => { s.board[28]=1; }], // En passant square can become legal next turn.
    [4,6, () => {}] // Castling rights and path are checked by Rust on the next turn.
  ]) {
    await waitForComputer(true, edit); const before = moveRequests().length;
    startDrag(from, to); drop(to); assert.equal(moveRequests().length, before);
    assert(square(to).classes.has('premoved'));
    await computerReply([[from,to,0]]); assert.equal(moveRequests().length, before + 1);
  }
  // A changed position can make a queued move illegal or capture its source.
  for (const [legal, edit] of [
    [[], () => {}],
    [[[6,21,0]], s => { s.board[6]=-4; }],
    [[[6,21,0]], s => { s.result='Black wins by checkmate'; }],
    [[[6,21,0]], s => { s.clocks.enabled=true; s.clocks.white_ms=0; }],
    [[[6,21,0]], s => { s.revision++; }],
    [[[6,21,0]], s => { s.notation=['d4','d5']; }],
    [[[6,21,0]], s => { s.notation.pop(); }],
    [[[6,21,0]], s => { s.human=null; }]
  ]) {
    await waitForComputer(); const before = moveRequests().length;
    clickSquare(6); clickSquare(21); clickSquare(21); clickSquare(38); await computerReply(legal, edit);
    assert.equal(moveRequests().length, before); assert(elements['cancel-premove'].hidden);
    assert(!square(6).classes.has('premoved'));
  }
  // Explicit cancellation, undo, resignation, and leaving the game discard queued intent.
  for (const cancel of [
    () => documentListeners.keydown({key:'Escape'}),
    () => elements.board.listeners.contextmenu({preventDefault() {}}),
    () => elements['cancel-premove'].onclick(),
    async () => { await elements.undo.onclick(); },
    () => { elements.resign.onclick(); elements['resign-dialog'].close(); },
    async () => { openMenu(); await submit(); }
  ]) {
    await waitForComputer(); const before = moveRequests().length;
    clickSquare(6); clickSquare(21); clickSquare(21); clickSquare(38); await cancel(); await computerReply([[6,21,0]]);
    assert.equal(moveRequests().length, before); assert(elements['cancel-premove'].hidden);
  }
  // Flipping preserves an intentional queue; an opponent reply during a drag cannot queue it late.
  await waitForComputer(); clickSquare(6); clickSquare(21); elements.flip.onclick();
  assert(square(21).classes.has('premoved')); const beforeFlip = moveRequests().length;
  await computerReply([[6,21,0]]); assert.equal(moveRequests().length, beforeFlip + 1);
  await waitForComputer(); startDrag(6, 21); const beforeInterruptedDrag = moveRequests().length;
  await computerReply([[6,21,0]]); drop(21); await tick();
  assert.equal(documentBody.children.length, 0); assert.equal(moveRequests().length, beforeInterruptedDrag);
  // Choose the human's promotion piece while waiting, including an underpromotion.
  await waitForComputer(true, s => { s.board[48]=1; s.board[56]=0; });
  const beforeQueuedPromotion = moveRequests().length;
  startDrag(48,56); drop(56); assert(elements['promotion-dialog'].open);
  assert(elements['promotion-choices'].children[0].innerHTML.includes('fill="#fffaf0"'));
  elements['promotion-choices'].children[3].onclick();
  assert(square(56).classes.has('premoved')); assert.equal(moveRequests().length, beforeQueuedPromotion);
  await computerReply([[48,56,5],[48,56,2]]);
  assert.equal(moveRequests().length, beforeQueuedPromotion + 1);
  assert(moveRequests().at(-1).url.includes('from=48&to=56&promotion=2'));
  await waitForComputer(true, s => { s.board[48]=1; s.board[56]=0; });
  startDrag(48,56); drop(56); const beforeInterruptedPromotion = moveRequests().length;
  await computerReply([[48,56,5]]); assert(!elements['promotion-dialog'].open);
  assert.equal(moveRequests().length, beforeInterruptedPromotion);
  // Stack three moves for one pawn using the projected board, without changing the real board.
  await waitForComputer(); const beforePawnLine = moveRequests().length;
  startDrag(11,27); drop(27); // d2-d4
  assert(square(11).attrs['aria-label'].includes('empty'));
  assert(square(27).attrs['aria-label'].includes('White pawn'));
  clickSquare(27); clickSquare(35); // d4-d5
  startDrag(35,43, {pointerType:'touch'}); drop(43, {pointerType:'touch'}); // d5-d6
  assert.equal(elements['premove-queue'].children.length, 3);
  assert.deepEqual(elements['premove-queue'].children.map(e => e.textContent), ['d2 → d4','d4 → d5','d5 → d6']);
  assert.equal(vm.runInContext('state.board[11]', context), 1);
  assert.equal(vm.runInContext('state.board[43]', context), 0);
  assert(square(43).attrs['aria-label'].includes('White pawn'));
  clickSquare(24); // An unrelated empty square just clears the selection.
  assert.equal(elements['premove-queue'].children.length, 3);
  await computerReply([[11,27,0]]);
  assert.equal(moveRequests().length, beforePawnLine + 1); assert.equal(elements['premove-queue'].children.length, 2);
  await vm.runInContext('poll()', context); await tick();
  assert.equal(moveRequests().length, beforePawnLine + 1);
  await computerReply([[27,35,0]], () => {}, [62,45]);
  assert.equal(moveRequests().length, beforePawnLine + 2); assert.equal(elements['premove-queue'].children.length, 1);
  await computerReply([[35,43,0]], () => {}, [57,42]);
  assert.equal(moveRequests().length, beforePawnLine + 3); assert(elements['premove-queue'].hidden);
  assert.deepEqual(moveRequests().slice(beforePawnLine).map(r => {
    const params = new URL(r.url, 'http://localhost').searchParams;
    return [Number(params.get('from')),Number(params.get('to'))];
  }), [[11,27],[27,35],[35,43]]);
  // Stop at an illegal middle move, even when a later move would be legal.
  await waitForComputer(); const beforeBrokenLine = moveRequests().length;
  startDrag(11,27); drop(27); startDrag(27,35); drop(35); startDrag(6,21); drop(21);
  await computerReply([[11,27,0]]); assert.equal(elements['premove-queue'].children.length, 2);
  await computerReply([[6,21,0]], () => {}, [62,45]);
  assert.equal(moveRequests().length, beforeBrokenLine + 1); assert(elements['premove-queue'].hidden);
  assert(!square(21).classes.has('premoved')); assert.equal(vm.runInContext('state.board[27]', context), 1);
  // A captured piece later in the line does not skip the still-legal head.
  await waitForComputer(); const beforeCapturedTail = moveRequests().length;
  startDrag(11,27); drop(27); startDrag(6,21); drop(21); startDrag(27,35); drop(35);
  await computerReply([[11,27,0]]);
  await computerReply([[6,21,0]], () => {}, [36,27]);
  assert.equal(moveRequests().length, beforeCapturedTail + 2); assert.equal(elements['premove-queue'].children.length, 1);
  await computerReply([[27,35,0]], () => {}, [62,45]);
  assert.equal(moveRequests().length, beforeCapturedTail + 2); assert(elements['premove-queue'].hidden);
  // Starting from a vacated original square resets the entire line, for clicks and drags.
  for (const restart of [
    () => { clickSquare(11); clickSquare(19); },
    () => { startDrag(11,19); drop(19); }
  ]) {
    await waitForComputer(); const before = moveRequests().length;
    startDrag(11,27); drop(27); startDrag(27,35); drop(35); startDrag(6,21); drop(21);
    restart(); assert.equal(elements['premove-queue'].children.length, 1);
    assert.equal(elements['premove-queue'].children[0].textContent, 'd2 → d3');
    assert(square(19).attrs['aria-label'].includes('White pawn')); assert(!square(35).classes.has('premoved'));
    assert(square(6).attrs['aria-label'].includes('White knight'));
    await computerReply([[11,19,0],[11,27,0],[6,21,0]]);
    assert.equal(moveRequests().length, before + 1); assert(moveRequests().at(-1).url.includes('from=11&to=19&promotion=0'));
    assert(elements['premove-queue'].hidden);
  }
  // New premoves append after the remaining line, including after one move executes.
  await waitForComputer(); const beforeAppend = moveRequests().length;
  startDrag(11,27); drop(27); startDrag(6,21); drop(21);
  await computerReply([[11,27,0]]);
  startDrag(27,35); drop(35); assert.equal(elements['premove-queue'].children.length, 2);
  await computerReply([[6,21,0],[27,35,0]], () => {}, [62,45]);
  assert(moveRequests().at(-1).url.includes('from=6&to=21&promotion=0'));
  await computerReply([[27,35,0]], () => {}, [57,42]);
  assert.equal(moveRequests().length, beforeAppend + 3); assert(elements['premove-queue'].hidden);
  // Projection includes promotion and the rook's castling move, allowing follow-up moves.
  for (const [edit, from, to, nextFrom, nextTo, legal] of [
    [s => { s.board[48]=1; s.board[56]=0; },48,56,56,41,[48,56,2]],
    [s => { s.board[5]=0; s.board[6]=0; },4,6,5,21,[4,6,0]],
    [s => { s.board[28]=0; s.board[36]=1; },36,45,45,53,[36,45,0]]
  ]) {
    await waitForComputer(true, edit); const before = moveRequests().length;
    startDrag(from,to); drop(to);
    if (elements['promotion-dialog'].open) elements['promotion-choices'].children[3].onclick();
    startDrag(nextFrom,nextTo); drop(nextTo); assert.equal(elements['premove-queue'].children.length, 2);
    await computerReply([legal], () => {}, from === 36 ? [53,37] : [52,36]);
    assert.equal(moveRequests().length, before + 1);
    if (from === 36) assert.equal(vm.runInContext('state.board[37]', context), 0); // En passant captured the pawn.
    await computerReply([[nextFrom,nextTo,0]], () => {}, [57,42]);
    assert.equal(moveRequests().length, before + 2); assert(elements['premove-queue'].hidden);
  }
  // A forced computer reply may arrive inside the accepted move response.
  await waitForComputer(); const beforeFastReply = moveRequests().length;
  startDrag(6,21); drop(21); startDrag(21,38); drop(38); startDrag(38,55); drop(55);
  afterNextMove = () => {
    server.revision++; server.notation.push('Nf6'); server.white=true; server.thinking=false; server.pondering=true;
    server.board[45]=server.board[62]; server.board[62]=0; server.legal=[[21,38,0]];
  };
  await computerReply([[6,21,0]]);
  assert.equal(moveRequests().length, beforeFastReply + 2); assert.equal(elements['premove-queue'].children.length, 1);
  await computerReply([[38,55,0]], () => {}, [57,42]);
  assert.equal(moveRequests().length, beforeFastReply + 3); assert(elements['premove-queue'].hidden);
  // Clearing the queue while its head is being submitted must not resurrect its tail.
  await waitForComputer(); const beforeInFlightCancel = moveRequests().length;
  startDrag(6,21); drop(21); startDrag(21,38); drop(38);
  afterNextMove = () => elements['cancel-premove'].onclick();
  await computerReply([[6,21,0]]); await computerReply([[21,38,0]], () => {}, [62,45]);
  assert.equal(moveRequests().length, beforeInFlightCancel + 1); assert(elements['premove-queue'].hidden);
  // A rejected automatic move is shown once and never retried as a premove.
  await waitForComputer(); clickSquare(6); clickSquare(21); clickSquare(21); clickSquare(38); rejectNextMove = true;
  const beforeRejectedPremove = moveRequests().length;
  await computerReply([[6,21,0]]); assert(!elements.error.hidden);
  await vm.runInContext('poll()', context); await tick();
  assert.equal(moveRequests().length, beforeRejectedPremove + 1); assert(elements['cancel-premove'].hidden);
  console.log('UI harness passed: controls, clocks, click/drag moves, ordered premove queues, projected pieces, restarts, illegal middle moves, special moves, cancellation, stale positions, turn guards, diagnostics, and sounds.');
})().catch(e => {console.error(e); process.exitCode = 1;});
