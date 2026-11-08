'use strict';
// Rendering and input only. Rust is the authority for every legal move.
const $ = id => document.getElementById(id);
const names = ['', 'pawn', 'knight', 'bishop', 'rook', 'queen', 'king'];
const timeControls = {'1': [60, 0], '2+1': [120, 1], '5': [300, 0], '10+5': [600, 5]};
function controlLabel(clocks) {
  if (!clocks.enabled) return 'Unlimited';
  return `${clocks.base_seconds / 60} min${clocks.increment_seconds ? ' + ' + clocks.increment_seconds + ' sec' : ''}`;
}
const shapes = {
  1: '<circle cx="50" cy="27" r="12"/><path d="M40 39h20l-5 9c0 11 5 19 12 25H33c7-6 12-14 12-25z"/><path d="M31 74h38v9H31zM27 84h46v7H27z"/>',
  2: '<path d="M29 75c1-14 6-21 19-30l-12 4-9-2-5-10 18-18 1-10 10 9c22-1 30 14 26 34l-5 23z"/><path d="M48 22c12 2 17 10 15 21-1 10-9 15-11 24" fill="none"/><path d="m26 37 9 3m1-11 6 1" fill="none"/><circle cx="48" cy="29" r="2" fill="currentColor" stroke="none"/><path d="M27 76h47v8H27zM23 85h55v7H23z"/>',
  3: '<path d="M50 12c-5 9-19 16-19 29 0 10 8 16 19 16s19-6 19-16c0-13-14-20-19-29z"/><path d="m53 28-10 15" fill="none"/><path d="M40 57h20l-3 10 10 9H33l10-9zM30 77h40v7H30zM25 85h50v7H25z"/><circle cx="50" cy="9" r="4"/>',
  4: '<path d="M27 16h12v10h7V16h9v10h7V16h12v22l-9 8v24l6 6H30l6-6V46l-9-8z"/><path d="M35 38h31M36 48h29M36 68h29" fill="none"/><path d="M29 77h43v8H29zM24 86h53v7H24z"/>',
  5: '<path d="m25 28 14 14 11-22 11 22 14-14-9 34H34z"/><path d="M34 62h32v7H34l-4 8h40l-4-8M29 78h42v7H29zM24 86h52v7H24z"/><circle cx="24" cy="24" r="5"/><circle cx="50" cy="16" r="5"/><circle cx="76" cy="24" r="5"/><path d="M36 55h28" fill="none"/>',
  6: '<path d="M46 9h8v9h9v8h-9v10h-8V26h-9v-8h9z"/><path d="M50 36c-14-14-30-1-23 12l10 17h26l10-17c7-13-9-26-23-12z"/><path d="M50 38v20M34 57h32" fill="none"/><path d="M36 65h28l3 11H33zM30 77h40v8H30zM25 86h50v7H25z"/>'
};
function piece(pc) {
  const white = pc > 0;
  return `<svg class="piece" viewBox="0 0 100 100" aria-hidden="true" focusable="false" fill="${white ? '#fffaf0' : '#263e33'}" stroke="${white ? '#344b3e' : '#e5e9d6'}" color="${white ? '#344b3e' : '#e5e9d6'}" stroke-width="2.3" stroke-linejoin="round" stroke-linecap="round">${shapes[Math.abs(pc)]}</svg>`;
}
// Recycle a fixed set of pieces through CSS animation instead of continually
// adding nodes. Negative delays make the falling background full immediately.
const fallingPieces = document.createDocumentFragment();
for (let i = 0; i < 24; i++) {
  const particle = document.createElement('span');
  const angle = (i * 43) % 50 - 25;
  particle.className = 'falling-piece';
  particle.innerHTML = piece((Math.floor(i / 6) % 2 ? 1 : -1) * (i % 6 + 1));
  particle.style.setProperty('--x', `${(i * 37 + 3) % 100}vw`);
  particle.style.setProperty('--size', `${24 + (i * 17) % 42}px`);
  particle.style.setProperty('--duration', `${16 + (i * 7) % 18}s`);
  particle.style.setProperty('--delay', `-${(i * 11) % 34}s`);
  particle.style.setProperty('--drift', `${(i * 29) % 73 - 36}px`);
  particle.style.setProperty('--start-angle', `${angle}deg`);
  particle.style.setProperty('--end-angle', `${angle + (i % 2 ? 90 : -90)}deg`);
  fallingPieces.append(particle);
}
$('menu-background').append(fallingPieces);
function squareName(s) { return String.fromCharCode(97 + s % 8) + (Math.floor(s / 8) + 1); }
let state = null, selected = null, flipped = false, playing = false, busy = false;
let lastHistoryLength = -1, lastStatus = '', polling = false;
let focusSquare = 12;
let soundEnabled = true, audioContext = null, soundEpoch = 0, soundSnapshot = null;
const voices = new Set();
try { soundEnabled = window.localStorage.getItem('chess.sounds') !== 'off'; } catch { /* Storage is optional. */ }
function updateSoundButton() {
  $('sound-toggle').textContent = soundEnabled ? 'Sound' : 'Muted';
  $('sound-toggle').setAttribute('aria-pressed', String(soundEnabled));
  $('sound-toggle').title = soundEnabled ? 'Mute game sounds' : 'Enable game sounds';
}
function stopVoices() {
  for (const voice of voices) {
    try { voice.oscillator.stop(); } catch { /* It may already have ended. */ }
    voice.oscillator.disconnect(); voice.gain.disconnect();
  }
  voices.clear();
}
function cancelSounds() { soundEpoch++; stopVoices(); }
// Create/resume audio during a click or keyboard activation, before any fetch.
// A blocked or unavailable audio device must never interrupt the game.
async function unlockAudio() {
  if (!soundEnabled) return false;
  try {
    if (!audioContext) {
      const Audio = window.AudioContext || window.webkitAudioContext;
      if (!Audio) return false;
      audioContext = new Audio();
    }
    if (audioContext.state !== 'running') await audioContext.resume();
    return audioContext.state === 'running';
  } catch { return false; }
}
const soundNotes = {
  move: [[0, 0.075, 640, 190, 0.13]],
  check: [[0, 0.12, 660, 660, 0.08], [0.15, 0.15, 880, 880, 0.08]],
  end: [[0, 0.16, 660, 660, 0.07], [0.18, 0.16, 523.25, 523.25, 0.07], [0.36, 0.24, 392, 392, 0.07]]
};
async function playSound(kind) {
  if (!soundEnabled || !audioContext) return;
  const epoch = soundEpoch;
  if (!await unlockAudio() || !soundEnabled || epoch !== soundEpoch) return;
  try {
    stopVoices();
    const start = audioContext.currentTime + 0.005;
    for (const [offset, duration, frequency, endFrequency, volume] of soundNotes[kind]) {
      const oscillator = audioContext.createOscillator(), gain = audioContext.createGain();
      const time = start + offset, voice = {oscillator, gain};
      oscillator.type = 'triangle';
      oscillator.frequency.setValueAtTime(frequency, time);
      oscillator.frequency.exponentialRampToValueAtTime(endFrequency, time + duration);
      gain.gain.setValueAtTime(0.0001, time);
      gain.gain.exponentialRampToValueAtTime(volume, time + 0.005);
      gain.gain.exponentialRampToValueAtTime(0.0001, time + duration);
      oscillator.connect(gain); gain.connect(audioContext.destination);
      voices.add(voice);
      oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); voices.delete(voice); };
      oscillator.start(time); oscillator.stop(time + duration + 0.01);
    }
  } catch { cancelSounds(); }
}
function resetSoundSnapshot(snapshot) {
  cancelSounds(); soundSnapshot = {notation: snapshot.notation.slice(), result: snapshot.result};
}
function announceGameChanges() {
  if (soundSnapshot) {
    const moved = state.notation.length > soundSnapshot.notation.length
      && soundSnapshot.notation.every((move, i) => state.notation[i] === move);
    // Play the most specific cue once. Checkmate gets the end cue; clocks,
    // selections, and repeated state responses never replay a move sound.
    if (state.result && !soundSnapshot.result) void playSound('end');
    else if (moved) void playSound(state.check ? 'check' : 'move');
  }
  soundSnapshot = {notation: state.notation.slice(), result: state.result};
}
updateSoundButton();
$('sound-toggle').onclick = () => {
  soundEnabled = !soundEnabled; cancelSounds(); updateSoundButton();
  try { window.localStorage.setItem('chess.sounds', soundEnabled ? 'on' : 'off'); } catch { /* Storage is optional. */ }
  if (soundEnabled) void unlockAudio();
};
function showError(error) { $('error-text').textContent = error.message || String(error); $('error').hidden = false; }
$('dismiss-error').onclick = () => { $('error').hidden = true; };
async function api(path, params) {
  const options = { cache: 'no-store', signal: AbortSignal.timeout(10000) };
  if (params) { options.method = 'POST'; options.headers = { 'X-Chess-Request': '1' }; }
  let response;
  try { response = await fetch('/api/' + path + (params ? '?' + new URLSearchParams(params) : ''), options); }
  catch { throw new Error('Cannot reach the chess server. Check that cargo run is still running, then try again.'); }
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || 'Something went wrong. Please try again.');
  data.receivedAt = performance.now();
  return data;
}
function changeMode() {
  const solo = document.querySelector('input[name="mode"]:checked').value === 'one';
  $('solo-options').hidden = !solo;
  changeTimeControl();
}
document.querySelectorAll('input[name="mode"]').forEach(el => el.addEventListener('change', changeMode));
function changeTimeControl() {
  const custom = $('time-control').value === 'custom';
  $('custom-time').hidden = !custom;
  $('base-minutes').disabled = !custom;
  $('increment-seconds').disabled = !custom;
  const thinking = $('time-control').value === 'unlimited' && document.querySelector('input[name="mode"]:checked').value === 'one';
  $('unlimited-thinking').hidden = !thinking;
  $('thinking-time').disabled = !thinking;
  const customThinking = thinking && $('thinking-time').value === 'custom';
  $('custom-thinking').hidden = !customThinking;
  $('thinking-seconds').disabled = !customThinking;
}
$('time-control').addEventListener('change', changeTimeControl);
$('thinking-time').addEventListener('change', changeTimeControl);
changeMode();
$('setup-form').addEventListener('submit', async e => {
  e.preventDefault(); if (busy) return;
  if (state && !state.result && state.notation.length && !window.confirm('Start a new game and replace the current one?')) return;
  void unlockAudio();
  busy = true; $('start').disabled = true; $('error').hidden = true;
  try {
    const form = new FormData(e.currentTarget);
    const control = form.get('time-control');
    const params = {mode: form.get('mode'), color: form.get('color')};
    if (control === 'unlimited') {
      params.unlimited = true;
      if (params.mode === 'one') params.thinking_seconds = Number(form.get('thinking-time') === 'custom' ? form.get('thinking-seconds') : form.get('thinking-time'));
    } else {
      const [base, increment] = control === 'custom'
        ? [Math.round(Number(form.get('minutes')) * 60), Number(form.get('increment'))] : timeControls[control];
      Object.assign(params, {base_seconds: base, increment_seconds: increment});
    }
    const next = await api('new', params);
    flipped = next.human === false; selected = null; lastHistoryLength = -1; lastStatus = '';
    $('engine-info').open = false;
    state = next; enterGame({fresh: true});
  } catch (error) { showError(error); }
  finally { busy = false; $('start').disabled = false; }
});
function enterGame({fresh = false} = {}) {
  playing = true; $('menu').hidden = true; $('game').hidden = false;
  resetSoundSnapshot(fresh ? {notation: [], result: null} : state);
  document.body.classList.add('in-game'); render();
  $('board').querySelector(`[data-square="${flipped ? 52 : 12}"]`)?.focus({preventScroll:true});
  window.scrollTo({top:0, behavior:'instant'});
}
$('menu-button').onclick = () => {
  playing = false; selected = null; $('menu').hidden = false; $('game').hidden = true;
  $('resume').hidden = false; document.body.classList.remove('in-game'); $('start').focus({preventScroll:true});
  window.scrollTo({top:0, behavior:'instant'});
};
$('resume').onclick = async () => {
  void unlockAudio();
  try { state = await api('state'); enterGame(); } catch (error) { showError(error); }
};
function clockRemaining(white, now = performance.now()) {
  const clocks = state.clocks;
  if (!clocks.enabled) return Infinity;
  const elapsed = clocks.running && clocks.active_white === white ? Math.max(0, now - state.receivedAt) : 0;
  return Math.max(0, (white ? clocks.white_ms : clocks.black_ms) - elapsed);
}
function renderClocks() {
  if (!playing || !state) return;
  $('game-clocks').hidden = !state.clocks.enabled;
  if (!state.clocks.enabled) return;
  const now = performance.now();
  for (const white of [true, false]) {
    const side = white ? 'white' : 'black', remaining = clockRemaining(white, now);
    const seconds = Math.ceil(remaining / 1000);
    const text = remaining < 10000 ? (Math.ceil(remaining / 100) / 10).toFixed(1)
      : `${Math.floor(seconds / 60)}:${String(seconds % 60).padStart(2, '0')}`;
    $(side + '-time').textContent = text;
    $(side + '-clock-label').textContent = `${white ? 'White' : 'Black'}${state.human === null ? '' : state.human === white ? ' · You' : ' · Computer'}`;
    $('clock-' + side).classList.toggle('active', state.clocks.running && state.clocks.active_white === white);
    $('clock-' + side).classList.toggle('low-time', remaining < 10000);
  }
}
setInterval(renderClocks, 100);
function canPlay() { return state && !busy && !state.thinking && !state.result && clockRemaining(state.white) > 0 && (state.human === null || state.human === state.white); }
function renderBoard() {
  const focusWasBoard = $('board').contains(document.activeElement);
  if (focusWasBoard) focusSquare = Number(document.activeElement.dataset.square);
  const targets = new Set(selected === null ? [] : state.legal.filter(m => m[0] === selected).map(m => m[1]));
  const fragment = document.createDocumentFragment();
  for (let r = 0; r < 8; r++) for (let c = 0; c < 8; c++) {
    const row = flipped ? r : 7 - r, col = flipped ? 7 - c : c, s = row * 8 + col, pc = state.board[s];
    const el = document.createElement('button');
    el.type = 'button'; el.className = `square ${(row + col) % 2 ? 'light' : 'dark'}`;
    el.dataset.square = s; el.tabIndex = s === focusSquare ? 0 : -1;
    if (state.last?.includes(s)) el.classList.add('last');
    if (selected === s) el.classList.add('selected');
    if (targets.has(s)) el.classList.add('target');
    if (pc) el.classList.add('occupied');
    if (state.check && pc === (state.white ? 6 : -6)) el.classList.add('check');
    const description = pc ? `${pc > 0 ? 'White' : 'Black'} ${names[Math.abs(pc)]}` : 'empty';
    el.setAttribute('aria-label', `${squareName(s)}, ${description}${targets.has(s) ? ', legal destination' : ''}`);
    el.setAttribute('aria-pressed', String(selected === s));
    if (pc) el.innerHTML = piece(pc);
    if (c === 0) { const coord = document.createElement('span'); coord.className = 'coord rank'; coord.textContent = row + 1; coord.setAttribute('aria-hidden','true'); el.append(coord); }
    if (r === 7) { const coord = document.createElement('span'); coord.className = 'coord file'; coord.textContent = String.fromCharCode(97 + col); coord.setAttribute('aria-hidden','true'); el.append(coord); }
    fragment.append(el);
  }
  $('board').replaceChildren(fragment);
  if (focusWasBoard) $('board').querySelector(`[data-square="${focusSquare}"]`)?.focus({preventScroll:true});
}
$('board').addEventListener('click', e => {
  const button = e.target.closest('[data-square]'); if (!button || !canPlay()) return;
  void unlockAudio();
  const s = Number(button.dataset.square); focusSquare = s;
  if (selected !== null) {
    const matching = state.legal.filter(m => m[0] === selected && m[1] === s);
    if (matching.length) {
      if (matching.some(m => m[2])) { choosePromotion(matching); return; }
      void makeMove(matching[0]); return;
    }
  }
  selected = selected === s ? null : (state.board[s] && (state.board[s] > 0) === state.white ? s : null);
  renderBoard();
});
$('board').addEventListener('keydown', e => {
  const keys = {ArrowUp:-8, ArrowDown:8, ArrowLeft:-1, ArrowRight:1};
  if (e.key === 'Escape') { selected = null; renderBoard(); return; }
  if (!(e.key in keys)) return;
  e.preventDefault();
  const buttons = [...$('board').children], index = buttons.indexOf(e.target);
  if (index < 0) return;
  if (e.key === 'ArrowLeft' && index % 8 === 0 || e.key === 'ArrowRight' && index % 8 === 7) return;
  const next = buttons[index + keys[e.key]];
  if (next) { e.target.tabIndex = -1; next.tabIndex = 0; focusSquare = Number(next.dataset.square); next.focus(); }
});
async function makeMove(move) {
  if (busy) return;
  void unlockAudio();
  busy = true; selected = null; $('error').hidden = true;
  try { state = await api('move', {from:move[0], to:move[1], promotion:move[2], revision:state.revision}); }
  catch (error) { showError(error); try { state = await api('state'); } catch { /* Keep the current board available. */ } }
  finally { busy = false; render(); }
}
function choosePromotion(moves) {
  $('promotion-choices').replaceChildren();
  for (const m of moves) {
    const button = document.createElement('button'); button.type = 'button';
    button.innerHTML = piece((state.white ? 1 : -1) * m[2]) + `<span>${names[m[2]][0].toUpperCase() + names[m[2]].slice(1)}</span>`;
    button.onclick = () => { $('promotion-dialog').close(); void makeMove(m); };
    $('promotion-choices').append(button);
  }
  $('promotion-dialog').showModal();
}
function render() {
  if (!playing || !state) return;
  announceGameChanges();
  renderBoard();
  renderClocks();
  const signature = [state.white, state.thinking, state.result, state.check, state.human, controlLabel(state.clocks), state.thinking_seconds].join('|');
  if (signature !== lastStatus) {
    const side = state.white ? 'White' : 'Black';
    $('status').textContent = state.result || (state.thinking ? 'Computer thinking…' : state.check ? `${side} is in check.` : `${side} to move.`);
    $('game-mode').textContent = `${state.human === null ? 'Two players' : 'You: ' + (state.human ? 'White' : 'Black')} · ${controlLabel(state.clocks)}${!state.clocks.enabled && state.human !== null ? ' · ' + state.thinking_seconds + 's per move' : ''}`;
    lastStatus = signature;
  }
  if (state.notation.length !== lastHistoryLength) {
    const fragment = document.createDocumentFragment();
    for (let i = 0; i < state.notation.length; i += 2) {
      const row = document.createElement('tr');
      for (const value of [String(i / 2 + 1) + '.', state.notation[i], state.notation[i+1] || '—']) { const td = document.createElement('td'); td.textContent = value; row.append(td); }
      fragment.append(row);
    }
    $('history').replaceChildren(fragment);
    $('history-table').hidden = !state.notation.length;
    $('history-empty').hidden = !!state.notation.length;
    $('history-scroll').scrollTop = $('history-scroll').scrollHeight; lastHistoryLength = state.notation.length;
  }
  $('undo').disabled = busy || !state.notation.length || state.human === false && state.notation.length === 1;
  $('resign').disabled = busy || !!state.result;
  renderEngine();
}
function renderEngine() {
  const analysis = state.human === null ? null : state.analysis;
  const stats = state.human === null ? null : analysis || state.stats;
  $('engine-status').textContent = state.human === null ? 'The computer is off in two-player mode.'
    : state.thinking ? 'Thinking… Live search.'
    : state.pondering ? 'Pondering possible replies during your turn.'
    : state.result ? 'Game finished.'
    : analysis?.pondering ? 'Pondering complete.'
    : stats ? 'Last computer move.' : 'Waiting for the first computer move.';
  $('engine-stats').hidden = !stats;
  if (stats) {
    $('engine-budget').textContent = stats.budget_millis === null ? 'Your turn' : `${(stats.budget_millis / 1000).toFixed(2)} s`;
    $('engine-depth').textContent = stats.depth;
    $('engine-threads').textContent = stats.threads || '—';
    $('engine-cache').textContent = stats.cache_limit ? `${(stats.cache_bytes / 1000000).toFixed(1)} / ${(stats.cache_limit / 1000000).toFixed(1)} MB` : '—';
    $('engine-searching-depth').textContent = analysis?.active && analysis.searching_depth ? analysis.searching_depth : '—';
    $('engine-nodes').textContent = stats.nodes.toLocaleString();
    $('engine-hits').textContent = stats.hits.toLocaleString();
    $('engine-time').textContent = `${(stats.millis / 1000).toFixed(2)} s`;
  }
}
$('flip').onclick = () => { flipped = !flipped; render(); };
$('undo').onclick = async () => {
  if (busy) return; busy = true; $('undo').disabled = true; selected = null;
  if ($('promotion-dialog').open) $('promotion-dialog').close();
  try { state = await api('undo', {}); resetSoundSnapshot(state); }
  catch (error) { showError(error); }
  finally { busy = false; render(); }
};
$('resign').onclick = () => {
  if (busy || state.result) return;
  const white = state.human === null ? state.white : state.human;
  $('resign-detail').textContent = `${white ? 'White' : 'Black'} will resign, and ${white ? 'Black' : 'White'} will win the game.`;
  $('resign-dialog').showModal();
};
$('confirm-resign').onclick = async () => {
  $('resign-dialog').close(); if (busy) return;
  busy = true; selected = null;
  void unlockAudio();
  try { state = await api('resign', {}); }
  catch (error) { showError(error); }
  finally { busy = false; render(); }
};
async function poll() {
  if (playing && !busy && !polling && !document.hidden) {
    polling = true;
    try {
      const next = await api('state');
      // Never overwrite a newer move response with an older polling response.
      if (!busy && (next.revision > state.revision || next.revision === state.revision && (next.thinking !== state.thinking || next.pondering !== state.pondering))) {
        if (next.revision !== state.revision) {
          selected = null;
          if ($('promotion-dialog').open) $('promotion-dialog').close();
        }
        state = next; render();
      } else if (!busy && next.revision === state.revision) {
        state.clocks = next.clocks; state.receivedAt = next.receivedAt;
        state.analysis = next.analysis; state.stats = next.stats;
        renderClocks(); renderEngine();
      }
    } catch (error) { showError(error); }
    finally { polling = false; }
  }
  setTimeout(poll, playing && (state?.thinking || state?.pondering || state?.clocks.running) ? 250 : 1000);
}
void poll();
// The Rust process owns the game, so a page refresh can resume it.
void (async () => {
  try {
    const existing = await api('state');
    if (!playing && !busy && (existing.notation.length || existing.thinking || existing.clocks.running || !existing.clocks.enabled || existing.result)) {
      state = existing; flipped = existing.human === false; $('resume').hidden = false;
    }
  } catch (error) { showError(error); }
})();
