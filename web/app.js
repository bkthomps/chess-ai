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
const backgroundParticles = [];
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
  backgroundParticles.push(particle);
}
$('menu-background').append(fallingPieces);
let backgroundPointer = null, backgroundFrame = false;
function moveBackgroundPieces() {
  backgroundFrame = false;
  if (document.body.classList.contains('in-game') && !document.body.classList.contains('disconnected')) return;
  if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const positions = backgroundParticles.map(particle => particle.getBoundingClientRect());
  backgroundParticles.forEach((particle, i) => {
    const rect = positions[i];
    const dx = backgroundPointer ? rect.left + rect.width / 2 - backgroundPointer.x : 0;
    const dy = backgroundPointer ? rect.top + rect.height / 2 - backgroundPointer.y : 0;
    const distance = Math.hypot(dx, dy);
    const strength = backgroundPointer && distance < 140 ? (1 - distance / 140) * 45 : 0;
    particle.style.setProperty('--push-x', `${distance ? dx / distance * strength : 0}px`);
    particle.style.setProperty('--push-y', `${distance ? dy / distance * strength : 0}px`);
  });
}
function scheduleBackgroundMove() {
  if (!backgroundFrame) {
    backgroundFrame = true;
    window.requestAnimationFrame(moveBackgroundPieces);
  }
}
window.addEventListener('pointermove', event => {
  if (event.pointerType !== 'mouse' && event.pointerType !== 'pen') return;
  backgroundPointer = {x: event.clientX, y: event.clientY};
  scheduleBackgroundMove();
});
window.addEventListener('pointerout', event => {
  if (event.relatedTarget) return;
  backgroundPointer = null;
  scheduleBackgroundMove();
});
window.addEventListener('click', event => {
  if (document.body.classList.contains('in-game') && !document.body.classList.contains('disconnected')) return;
  if (window.matchMedia && window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  if (event.target.closest('button, a, input, select, textarea, label')) return;
  for (const particle of backgroundParticles) {
    if (particle.classList.contains('popped')) continue;
    const visual = particle.querySelector('.piece');
    const rect = visual.getBoundingClientRect();
    if (event.clientX >= rect.left && event.clientX <= rect.right &&
        event.clientY >= rect.top && event.clientY <= rect.bottom) {
      particle.classList.add('popped');
      void unlockAudio().then(ready => { if (ready) void playSound('pop'); });
      break;
    }
  }
});
function squareName(s) { return String.fromCharCode(97 + s % 8) + (Math.floor(s / 8) + 1); }
let state = null, selected = null, flipped = false, playing = false, busy = false;
let confirmingNewGame = false;
let newGameConfirmationTimer = null;
let lastHistoryLength = -1, lastStatus = '', polling = false;
let focusSquare = 12;
let drag = null, suppressDragClick = false;
let premoves = [], premoveContext = null;
let soundEnabled = true, audioContext = null, soundEpoch = 0, soundSnapshot = null;
const voices = new Set();
try { soundEnabled = window.localStorage.getItem('chess.sounds') !== 'off'; } catch { /* Storage is optional. */ }
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
  pop: [[0, 0.07, 550, 120, 0.1]],
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
// Schedule bounded phrases so screen changes can stop all scheduled notes.
const musicVoices = new Set();
let musicTimer = null, musicEpoch = 0, musicActive = false, musicTheme = null;
const themes = {
  menu: {step: 0.32, notes: [
    261.63, 329.63, 392, 523.25, 493.88, 392, 440, 493.88,
    523.25, 392, 329.63, 392, 440, 349.23, 392, 0,
    329.63, 392, 523.25, 587.33, 659.25, 587.33, 523.25, 493.88,
    440, 523.25, 392, 440, 349.23, 392, 329.63, 261.63
  ]},
  disconnected: {step: 0.72, notes: [
    329.63, 293.66, 261.63, 246.94, 220, 0, 261.63, 246.94,
    220, 196, 220, 246.94, 261.63, 246.94, 220, 0
  ]}
};
function stopMusic() {
  musicEpoch++; musicActive = false; musicTheme = null;
  if (musicTimer !== null) clearTimeout(musicTimer);
  musicTimer = null;
  for (const voice of musicVoices) {
    try { voice.oscillator.stop(); } catch { /* It may already have ended. */ }
    voice.oscillator.disconnect(); voice.gain.disconnect();
  }
  musicVoices.clear();
}
function musicWanted() {
  return soundEnabled && !document.hidden && (!playing || document.body.classList.contains('disconnected'));
}
async function startMusic() {
  if (!musicWanted() || musicActive) return;
  const epoch = musicEpoch;
  if (!await unlockAudio() || epoch !== musicEpoch || !musicWanted() || musicActive) return;
  musicActive = true;
  musicTheme = document.body.classList.contains('disconnected') ? 'disconnected' : 'menu';
  const theme = themes[musicTheme];
  function phrase() {
    if (epoch !== musicEpoch || !musicWanted()) { stopMusic(); return; }
    try {
      const start = audioContext.currentTime + 0.03;
      theme.notes.forEach((frequency, i) => {
        if (!frequency) return;
        const time = start + i * theme.step;
        const oscillator = audioContext.createOscillator(), gain = audioContext.createGain();
        const voice = {oscillator, gain};
        oscillator.type = 'sine';
        oscillator.frequency.setValueAtTime(frequency, time);
        gain.gain.setValueAtTime(0.0001, time);
        gain.gain.exponentialRampToValueAtTime(0.025, time + 0.04);
        gain.gain.exponentialRampToValueAtTime(0.0001, time + theme.step * 0.9);
        oscillator.connect(gain); gain.connect(audioContext.destination);
        musicVoices.add(voice);
        oscillator.onended = () => { oscillator.disconnect(); gain.disconnect(); musicVoices.delete(voice); };
        oscillator.start(time); oscillator.stop(time + theme.step * 0.9 + 0.01);
      });
      musicTimer = setTimeout(phrase, theme.notes.length * theme.step * 1000);
    } catch { stopMusic(); }
  }
  phrase();
}
function syncMusic() {
  const nextTheme = document.body.classList.contains('disconnected') ? 'disconnected' : 'menu';
  if (!musicWanted() || (musicActive && musicTheme !== nextTheme)) stopMusic();
  if (musicWanted()) void startMusic();
}
// Autoplay policies may require a gesture before an AudioContext can run.
window.addEventListener('pointerdown', syncMusic);
window.addEventListener('keydown', syncMusic);
syncMusic();
function showError(error) {
  if (error.message === 'Cannot reach the chess server.') return;
  $('error-text').textContent = error.message || String(error); $('error').hidden = false;
}
$('dismiss-error').onclick = () => { $('error').hidden = true; };
async function api(path, params) {
  const options = { cache: 'no-store', signal: AbortSignal.timeout(10000) };
  if (params) { options.method = 'POST'; options.headers = { 'X-Chess-Request': '1' }; }
  let response;
  try { response = await fetch('/api/' + path + (params ? '?' + new URLSearchParams(params) : ''), options); }
  catch {
    if (!document.body.classList.contains('disconnected')) {
      const white = state ? (state.human === null ? state.white : state.human) : true;
      $('disconnected-king').innerHTML = piece(white ? 6 : -6).replace('</svg>', '<path d="M45 48l10 10m0-10L45 58" fill="none" stroke="#b33a3a" stroke-width="4"/></svg>');
    }
    document.body.classList.add('disconnected'); $('disconnected').hidden = false;
    cancelSounds(); syncMusic();
    for (const id of ['promotion-dialog', 'resign-dialog']) if ($(id).open) $(id).close();
    throw new Error('Cannot reach the chess server.');
  }
  const data = await response.json();
  if (!response.ok) throw new Error(data.error || 'Something went wrong. Please try again.');
  data.receivedAt = performance.now();
  return data;
}
function changeMode() {
  const solo = document.querySelector('input[name="mode"]:checked').value === 'one';
  $('solo-options').hidden = !solo;
  $('two-player-options').hidden = solo;
  $('two-player-mode').disabled = solo;
  changeTimeControl();
}
document.querySelectorAll('input[name="mode"]').forEach(el => el.addEventListener('change', changeMode));
function changeTimeControl() {
  const custom = $('time-control').value === 'custom';
  $('custom-time').hidden = !custom;
  $('base-minutes').disabled = !custom;
  $('increment-seconds').disabled = !custom;
  const thinking = $('time-control').value === 'unlimited' && (document.querySelector('input[name="mode"]:checked').value === 'one' || $('two-player-mode').value === 'ai');
  $('unlimited-thinking').hidden = !thinking;
  $('thinking-time').disabled = !thinking;
  const customThinking = thinking && $('thinking-time').value === 'custom';
  $('custom-thinking').hidden = !customThinking;
  $('thinking-seconds').disabled = !customThinking;
}
$('time-control').addEventListener('change', changeTimeControl);
$('thinking-time').addEventListener('change', changeTimeControl);
$('two-player-mode').addEventListener('change', changeTimeControl);
changeMode();
$('setup-form').addEventListener('submit', async e => {
  e.preventDefault(); if (busy) return;
  clearBoardInput();
  void unlockAudio();
  busy = true; $('start').disabled = true; $('error').hidden = true;
  try {
    const form = new FormData(e.currentTarget);
    const control = form.get('time-control');
    const params = {mode: form.get('mode'), color: form.get('color')};
    if (params.mode === 'two') params.two_player_mode = form.get('two-player-mode');
    if (control === 'unlimited') {
      params.unlimited = true;
      if (params.mode === 'one' || params.two_player_mode === 'ai') params.thinking_seconds = Number(form.get('thinking-time') === 'custom' ? form.get('thinking-seconds') : form.get('thinking-time'));
    } else {
      const [base, increment] = control === 'custom'
        ? [Math.round(Number(form.get('minutes')) * 60), Number(form.get('increment'))] : timeControls[control];
      Object.assign(params, {base_seconds: base, increment_seconds: increment});
    }
    const next = await api('new', params);
    flipped = next.human === false; selected = null; lastHistoryLength = -1; lastStatus = '';
    $('engine-info').open = false;
    state = next; enterGame();
  } catch (error) { showError(error); }
  finally { busy = false; $('start').disabled = false; }
});
function resetNewGameConfirmation() {
  if (newGameConfirmationTimer !== null) clearTimeout(newGameConfirmationTimer);
  newGameConfirmationTimer = null;
  confirmingNewGame = false; $('menu-button').textContent = 'New game'; $('menu-button').classList.remove('confirm');
}
document.addEventListener('click', e => {
  if (confirmingNewGame && !$('menu-button').contains(e.target)) resetNewGameConfirmation();
});
function enterGame() {
  resetNewGameConfirmation();
  playing = true; $('menu').hidden = true; $('game').hidden = false; syncMusic();
  resetSoundSnapshot({notation: [], result: null});
  document.body.classList.add('in-game'); render();
  $('board').querySelector(`[data-square="${flipped ? 52 : 12}"]`)?.focus({preventScroll:true});
  window.scrollTo({top:0, behavior:'instant'});
}
$('menu-button').onclick = () => {
  clearBoardInput();
  if (state && !state.result && !confirmingNewGame) {
    confirmingNewGame = true; $('menu-button').textContent = 'Confirm'; $('menu-button').classList.add('confirm');
    newGameConfirmationTimer = setTimeout(resetNewGameConfirmation, 5000);
    renderBoard();
    return;
  }
  resetNewGameConfirmation();
  playing = false; selected = null; $('menu').hidden = false; $('game').hidden = true;
  document.body.classList.remove('in-game'); syncMusic(); $('start').focus({preventScroll:true});
  window.scrollTo({top:0, behavior:'instant'});
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
    $(side + '-clock-label').textContent = `${white ? 'White' : 'Black'}${state.ai_vs_ai ? ' · Computer' : state.human === null ? '' : state.human === white ? ' · You' : ' · Computer'}`;
    $('clock-' + side).classList.toggle('active', state.clocks.running && state.clocks.active_white === white);
    $('clock-' + side).classList.toggle('low-time', remaining < 10000);
  }
}
setInterval(renderClocks, 100);
function canPlay() { return state && !state.ai_vs_ai && !busy && !state.thinking && !state.result && clockRemaining(state.white) > 0 && (state.human === null || state.human === state.white); }
function canPremove() {
  return state && !state.ai_vs_ai && playing && !busy && !state.result && state.human !== null && state.human !== state.white
    && clockRemaining(state.white) > 0 && clockRemaining(state.human) > 0;
}
function canInput() { return canPlay() || canPremove(); }
function inputColor() { return state.human === null ? state.white : state.human; }
// Preview only our queued moves. Opponent replies remain unknown until Rust
// publishes them; previewing a move never establishes that it is legal.
function premoveBoard() {
  const board = state.board.slice();
  for (const {move:[from, to, promotion], pc} of premoves) {
    if (board[from] !== pc) continue;
    if (Math.abs(pc) === 1 && from % 8 !== to % 8 && !board[to]) {
      const captured = to + (pc > 0 ? -8 : 8);
      if (board[captured] === -pc) board[captured] = 0;
    }
    if (Math.abs(pc) === 6 && Math.abs(to - from) === 2) {
      const rookFrom = from + (to > from ? 3 : -4), rookTo = from + (to > from ? 1 : -1);
      if (board[rookFrom] === (pc > 0 ? 4 : -4)) {
        board[rookTo] = board[rookFrom]; board[rookFrom] = 0;
      }
    }
    board[to] = promotion ? (pc > 0 ? promotion : -promotion) : pc;
    board[from] = 0;
  }
  return board;
}
function inputSource(square) {
  const pc = premoveBoard()[square];
  if (pc && (pc > 0) === inputColor()) return {pc, restart:false};
  // Selecting a real piece that earlier premoves moved away starts a new line.
  const actual = state.board[square];
  return canPremove() && actual && (actual > 0) === state.human ? {pc:actual, restart:true} : null;
}
// These are conditional destinations, not legal moves. Ignore occupancy and
// check: the opponent may vacate a square, capture a blocker, or enable a capture.
// Rust's next legal-move list is the authority before anything is submitted.
function inputMoves(from, board = premoveBoard()) {
  if (!canPremove()) return state.legal.filter(m => m[0] === from);
  const pc = board[from];
  if (!pc || (pc > 0) !== state.human) return [];
  const moves = [], rank = Math.floor(from / 8), file = from % 8, kind = Math.abs(pc);
  for (let to = 0; to < 64; to++) {
    if (to === from) continue;
    const dr = Math.floor(to / 8) - rank, df = to % 8 - file;
    const r = Math.abs(dr), f = Math.abs(df), forward = pc > 0 ? 1 : -1;
    let candidate = false;
    if (kind === 1) candidate = dr === forward && f <= 1 || df === 0 && dr === 2 * forward && rank === (pc > 0 ? 1 : 6);
    if (kind === 2) candidate = r * f === 2;
    if (kind === 3) candidate = r === f;
    if (kind === 4) candidate = dr === 0 || df === 0;
    if (kind === 5) candidate = r === f || dr === 0 || df === 0;
    if (kind === 6) candidate = Math.max(r, f) === 1 || from === (pc > 0 ? 4 : 60) && dr === 0 && f === 2;
    if (candidate) {
      const promotions = kind === 1 && Math.floor(to / 8) === (pc > 0 ? 7 : 0) ? [5, 4, 3, 2] : [0];
      for (const promotion of promotions) moves.push([from, to, promotion]);
    }
  }
  return moves;
}
function renderPremove() {
  const count = premoves.length;
  $('premove-controls').hidden = !count && !canPremove();
  $('cancel-premove').hidden = !count;
  $('premove-status').textContent = count ? `${count} premove${count === 1 ? '' : 's'} queued · Board preview`
    : 'You can queue premoves while the computer thinks.';
  $('premove-queue').hidden = !count;
  const fragment = document.createDocumentFragment();
  for (const {move:[from, to, promotion]} of premoves) {
    const item = document.createElement('li');
    item.textContent = `${squareName(from)} → ${squareName(to)}${promotion ? ' · ' + names[promotion] : ''}`;
    fragment.append(item);
  }
  $('premove-queue').replaceChildren(fragment);
}
function clearPremoves() { premoves = []; premoveContext = null; }
function clearBoardInput() {
  cancelDrag(); selected = null; clearPremoves();
  if ($('promotion-dialog').open) $('promotion-dialog').close();
  renderPremove();
}
function queuePremove(move, revision) {
  if (!canPremove() || state.revision !== revision) return;
  const pc = premoveBoard()[move[0]];
  if (!pc || (pc > 0) !== state.human) return;
  if (!premoves.length) premoveContext = {human:state.human, revision, notation:state.notation.slice()};
  premoves.push({move:move.slice(), pc});
  selected = null; renderBoard();
}
function takeReadyPremove() {
  if (!premoves.length || busy) return null;
  const context = premoveContext, queued = premoves[0];
  if (!state.result && state.human === context.human && state.revision === context.revision
      && clockRemaining(state.white) > 0 && clockRemaining(context.human) > 0) return null;
  // Validate only the head, after exactly one opponent reply. Later premoves
  // wait for their own turns, and the first failure discards the entire line.
  const next = canPlay() && state.human === context.human && state.white === context.human
    && state.revision === context.revision + 1 && state.board[queued.move[0]] === queued.pc
    && state.notation.length === context.notation.length + 1
    && context.notation.every((move, i) => state.notation[i] === move)
    && state.legal.find(m => m.every((value, i) => value === queued.move[i]));
  if (!next) clearPremoves();
  return next || null;
}
$('cancel-premove').onclick = () => { clearBoardInput(); renderBoard(); };
function renderBoard(keepDrag = false) {
  if (!keepDrag) cancelDrag();
  const focusWasBoard = $('board').contains(document.activeElement);
  if (focusWasBoard) focusSquare = Number(document.activeElement.dataset.square);
  const premoving = canPremove(), movable = canInput();
  const board = premoveBoard();
  const targets = new Set(selected === null ? [] : inputMoves(selected, board).map(m => m[1]));
  const queuedSquares = new Set(premoves.flatMap(({move}) => move.slice(0, 2)));
  const fragment = document.createDocumentFragment();
  for (let r = 0; r < 8; r++) for (let c = 0; c < 8; c++) {
    const row = flipped ? r : 7 - r, col = flipped ? 7 - c : c, s = row * 8 + col, pc = board[s];
    const el = document.createElement('button');
    el.type = 'button'; el.className = `square ${(row + col) % 2 ? 'light' : 'dark'}`;
    el.dataset.square = s; el.tabIndex = s === focusSquare ? 0 : -1;
    if (state.last?.includes(s)) el.classList.add('last');
    if (selected === s) el.classList.add('selected');
    if (targets.has(s)) el.classList.add('target');
    if (targets.has(s) && premoving) el.classList.add('premove-target');
    const queuedSquare = queuedSquares.has(s);
    if (queuedSquare) el.classList.add('premoved');
    if (pc) el.classList.add('occupied');
    if (pc && pc !== state.board[s]) el.classList.add('preview-piece');
    if (pc && (pc > 0) === inputColor() && movable) el.classList.add('movable');
    if (state.check && pc === (state.white ? 6 : -6)) el.classList.add('check');
    const description = pc ? `${pc > 0 ? 'White' : 'Black'} ${names[Math.abs(pc)]}` : 'empty';
    const destination = targets.has(s) ? premoving ? ', possible premove destination' : ', legal destination' : '';
    const restart = !pc && state.board[s] && (state.board[s] > 0) === state.human && premoves.length;
    if (restart && movable) el.classList.add('movable');
    el.setAttribute('aria-label', `${squareName(s)}, ${description}${destination}${queuedSquare ? ', queued premove' : ''}${restart ? ', select to restart premoves from the actual piece' : ''}`);
    el.setAttribute('aria-pressed', String(selected === s));
    if (pc) el.innerHTML = piece(pc);
    if (c === 0) { const coord = document.createElement('span'); coord.className = 'coord rank'; coord.textContent = row + 1; coord.setAttribute('aria-hidden','true'); el.append(coord); }
    if (r === 7) { const coord = document.createElement('span'); coord.className = 'coord file'; coord.textContent = String.fromCharCode(97 + col); coord.setAttribute('aria-hidden','true'); el.append(coord); }
    fragment.append(el);
  }
  $('board').replaceChildren(fragment);
  if (keepDrag && drag?.active) {
    const source = $('board').querySelector(`[data-square="${drag.from}"]`);
    source?.classList.add('drag-source');
    drag.over = squareAtPointer({clientX:drag.x, clientY:drag.y});
    if (drag.over?.classList.contains('target')) drag.over.classList.add('drag-over');
  }
  if (focusWasBoard) $('board').querySelector(`[data-square="${focusSquare}"]`)?.focus({preventScroll:true});
  renderPremove();
}
// Wait for deliberate movement so taps and small mouse movements remain clicks.
// Capture on the board only after dragging starts: square buttons are replaced
// when hints render, and capturing a simple press would retarget its click.
function cancelDrag() {
  if (!drag) return;
  const previous = drag; drag = null;
  if (previous.active) suppressDragClick = true;
  previous.ghost?.remove(); previous.over?.classList.remove('drag-over');
  $('board').querySelector(`[data-square="${previous.from}"]`)?.classList.remove('drag-source');
  $('board').classList.remove('dragging');
  if ($('board').hasPointerCapture(previous.id)) $('board').releasePointerCapture(previous.id);
}
function squareAtPointer(e) {
  const square = document.elementFromPoint(e.clientX, e.clientY)?.closest('[data-square]');
  return square && $('board').contains(square) ? square : null;
}
function tryMove(from, to) {
  const matching = inputMoves(from).filter(m => m[1] === to);
  if (!matching.length) return false;
  if (canPremove()) {
    const revision = state.revision;
    if (matching.some(m => m[2])) choosePromotion(matching, m => queuePremove(m, revision), inputColor());
    else queuePremove(matching[0], revision);
  } else if (matching.some(m => m[2])) choosePromotion(matching);
  else void makeMove(matching[0]);
  return true;
}
$('board').addEventListener('pointerdown', e => {
  if (!e.isPrimary || e.button !== 0 || drag) return;
  suppressDragClick = false;
  const button = e.target.closest('[data-square]');
  if (!button || !canInput()) return;
  const from = Number(button.dataset.square), source = inputSource(from);
  if (!source) return;
  drag = {id:e.pointerId, from, ...source, revision:state.revision,
    startX:e.clientX, startY:e.clientY, x:e.clientX, y:e.clientY, active:false};
});
document.addEventListener('pointermove', e => {
  if (!drag || e.pointerId !== drag.id) return;
  if (!canInput() || state.revision !== drag.revision) { cancelDrag(); return; }
  if (!drag.active) {
    if (Math.hypot(e.clientX - drag.x, e.clientY - drag.y) < 6) return;
    if (drag.restart) clearPremoves();
    drag.active = true; selected = drag.from; focusSquare = drag.from;
    // Transfer touch's implicit capture before replacing its original square.
    $('board').setPointerCapture(drag.id);
    renderBoard(true);
    const source = $('board').querySelector(`[data-square="${drag.from}"]`);
    const size = source.getBoundingClientRect().width;
    source.classList.add('drag-source'); source.focus({preventScroll:true});
    drag.ghost = document.createElement('div'); drag.ghost.className = 'drag-piece';
    drag.ghost.setAttribute('aria-hidden', 'true'); drag.ghost.innerHTML = piece(drag.pc);
    drag.ghost.style.width = `${size}px`; drag.ghost.style.height = `${size}px`;
    document.body.append(drag.ghost); $('board').classList.add('dragging');
    void unlockAudio();
  }
  e.preventDefault();
  drag.x = e.clientX; drag.y = e.clientY;
  drag.ghost.style.left = `${e.clientX}px`; drag.ghost.style.top = `${e.clientY}px`;
  drag.over?.classList.remove('drag-over');
  drag.over = squareAtPointer(e);
  if (drag.over?.classList.contains('target')) drag.over.classList.add('drag-over');
}, {passive:false});
document.addEventListener('pointerup', e => {
  if (!drag || e.pointerId !== drag.id) return;
  const previous = drag;
  // A fast gesture can reach pointerup before the browser dispatches a
  // pointermove. Treat clear down-to-up movement as a drag on release too.
  if (!previous.active && Math.hypot(e.clientX - previous.startX, e.clientY - previous.startY) >= 6) {
    previous.active = true;
  }
  const target = previous.active ? squareAtPointer(e) : null;
  cancelDrag();
  if (!previous.active) return;
  e.preventDefault();
  if (target && canInput() && state.revision === previous.revision) {
    const to = Number(target.dataset.square);
    focusSquare = to;
    if (tryMove(previous.from, to)) return;
  }
  renderBoard();
});
document.addEventListener('pointercancel', e => { if (drag?.id === e.pointerId) cancelDrag(); });
$('board').addEventListener('lostpointercapture', e => {
  if (e.target === $('board') && drag?.id === e.pointerId) cancelDrag();
});
$('board').addEventListener('dragstart', e => e.preventDefault());
window.addEventListener('blur', cancelDrag);
document.addEventListener('visibilitychange', () => { if (document.hidden) cancelDrag(); syncMusic(); });
$('board').addEventListener('click', e => {
  // A completed or cancelled drag may still produce a compatibility click.
  // Keyboard activation (detail 0) must always keep working.
  if (suppressDragClick && e.detail !== 0) { suppressDragClick = false; e.preventDefault(); return; }
  const button = e.target.closest('[data-square]'); if (!button || !canInput()) return;
  void unlockAudio();
  const s = Number(button.dataset.square); focusSquare = s;
  if (selected !== null && tryMove(selected, s)) return;
  const source = inputSource(s);
  if (source?.restart) clearPremoves();
  selected = selected === s ? null : source ? s : null;
  renderBoard();
});
$('board').addEventListener('keydown', e => {
  const keys = {ArrowUp:-8, ArrowDown:8, ArrowLeft:-1, ArrowRight:1};
  if (!(e.key in keys)) return;
  e.preventDefault();
  const buttons = [...$('board').children], index = buttons.indexOf(e.target);
  if (index < 0) return;
  if (e.key === 'ArrowLeft' && index % 8 === 0 || e.key === 'ArrowRight' && index % 8 === 7) return;
  const next = buttons[index + keys[e.key]];
  if (next) { e.target.tabIndex = -1; next.tabIndex = 0; focusSquare = Number(next.dataset.square); next.focus(); }
});
document.addEventListener('keydown', e => {
  if (e.key === 'Escape' && playing && !$('promotion-dialog').open && !$('resign-dialog').open) {
    clearBoardInput(); renderBoard();
  }
});
$('board').addEventListener('contextmenu', e => {
  if (!premoves.length && !canPremove()) return;
  e.preventDefault(); clearBoardInput(); renderBoard();
});
async function makeMove(move, fromQueue = false) {
  if (busy) return;
  if (fromQueue) { cancelDrag(); selected = null; }
  else clearBoardInput();
  const queue = premoves, previous = state;
  void unlockAudio();
  busy = true; selected = null; $('error').hidden = true;
  try {
    state = await api('move', {from:move[0], to:move[1], promotion:move[2], revision:previous.revision});
    if (fromQueue && premoves === queue) {
      // A forced computer reply can already be included in the move response.
      // Keep the context immediately after our accepted move in either case.
      const steps = state.revision - previous.revision;
      if (state.human === previous.human && (steps === 1 || steps === 2)
          && state.notation.length === previous.notation.length + steps
          && previous.notation.every((value, i) => state.notation[i] === value)) {
        premoves = premoves.slice(1);
        premoveContext = premoves.length ? {human:state.human, revision:previous.revision + 1,
          notation:state.notation.slice(0, previous.notation.length + 1)} : null;
      } else clearPremoves();
    }
  }
  catch (error) {
    clearPremoves(); showError(error);
    if (!document.body.classList.contains('disconnected')) {
      try { state = await api('state'); } catch { /* Keep the current board available. */ }
    }
  }
  finally { busy = false; render(); }
}
function choosePromotion(moves, submitMove = makeMove, white = state.white) {
  $('promotion-choices').replaceChildren();
  for (const m of moves) {
    const button = document.createElement('button'); button.type = 'button';
    button.innerHTML = piece((white ? 1 : -1) * m[2]) + `<span>${names[m[2]][0].toUpperCase() + names[m[2]].slice(1)}</span>`;
    button.onclick = () => { $('promotion-dialog').close(); void submitMove(m); };
    $('promotion-choices').append(button);
  }
  $('promotion-dialog').showModal();
}
function render() {
  if (!playing || !state) return;
  announceGameChanges();
  // A new opponent position invalidates the old move targets, but it should
  // not interrupt an in-progress drag when its source piece is still there.
  // Keep the pointer and ghost alive, then render the new legal destinations.
  if (drag && drag.revision !== state.revision) {
    if (drag.active && state.board[drag.from] === drag.pc && canInput()) {
      drag.revision = state.revision;
      selected = drag.from;
    } else cancelDrag();
  }
  const readyPremove = takeReadyPremove();
  renderBoard(!!drag?.active && state.revision === drag.revision && canInput());
  renderClocks();
  const signature = [state.white, state.thinking, state.result, state.check, state.human, state.ai_vs_ai, controlLabel(state.clocks), state.thinking_seconds].join('|');
  if (signature !== lastStatus) {
    const side = state.white ? 'White' : 'Black';
    $('status').textContent = state.result || (state.thinking ? (state.ai_vs_ai ? `${side} computer thinking…` : 'Computer thinking…') : state.check ? `${side} is in check.` : `${side} to move.`);
    $('game-mode').textContent = `${state.ai_vs_ai ? 'AI vs. AI' : state.human === null ? 'Two players' : 'You: ' + (state.human ? 'White' : 'Black')} · ${controlLabel(state.clocks)}${!state.clocks.enabled && (state.human !== null || state.ai_vs_ai) ? ' · ' + state.thinking_seconds + 's per move' : ''}`;
    lastStatus = signature;
  }
  $('moves-title').textContent = openingName(state.notation);
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
  $('resign').disabled = busy || !!state.result || state.ai_vs_ai;
  if (state.result && confirmingNewGame) resetNewGameConfirmation();
  renderEngine();
  if (readyPremove) void makeMove(readyPremove, true);
}
function renderEngine() {
  const analysis = state.human === null && !state.ai_vs_ai ? null : state.analysis;
  const stats = state.human === null && !state.ai_vs_ai ? null : analysis || state.stats;
  $('engine-status').textContent = state.human === null && !state.ai_vs_ai ? 'The computer is off in two-player mode.'
    : state.thinking ? 'Thinking… Live search.'
    : state.pondering ? 'Pondering possible replies.'
    : state.result ? 'Game finished.'
    : analysis?.pondering ? 'Pondering complete.'
    : stats ? 'Last computer move.' : 'Waiting for the first computer move.';
  $('engine-stats').hidden = !stats;
  if (stats) {
    const timeLimit = stats.budget_millis === null ? 'Your turn' : `${(stats.budget_millis / 1000).toFixed(2)} s`;
    const evaluation = stats.evaluation;
    $('engine-evaluation').textContent = evaluation == null ? '—' : Math.abs(evaluation) > 29744 ? (evaluation > 0 ? '+Mate' : '−Mate') : `${evaluation > 0 ? '+' : ''}${(evaluation / 100).toFixed(2)}`;
    $('engine-depth').textContent = stats.depth;
    $('engine-threads').textContent = stats.threads || '—';
    $('engine-cache').textContent = stats.cache_limit ? `${(stats.cache_bytes / 1000000).toFixed(1)} / ${(stats.cache_limit / 1000000).toFixed(1)} MB` : '—';
    $('engine-nodes').textContent = stats.nodes.toLocaleString();
    $('engine-hits').textContent = stats.hits.toLocaleString();
    $('engine-time').textContent = `${(stats.millis / 1000).toFixed(2)} s / ${timeLimit}`;
  }
}
$('flip').onclick = () => { flipped = !flipped; render(); };
let copyPgnTimer;
function gamePgn(game) {
  const result = !game.result ? '*' : game.result.startsWith('White wins') ? '1-0'
    : game.result.startsWith('Black wins') ? '0-1' : '1/2-1/2';
  const player = white => game.ai_vs_ai ? 'Computer' : game.human === null ? 'Player' : game.human === white ? 'Player' : 'Computer';
  const thinking = !game.clocks.enabled && (game.human !== null || game.ai_vs_ai)
    ? `, ${game.thinking_seconds} sec computer thinking time per move` : '';
  const tags = [`[Event "${controlLabel(game.clocks)}${thinking}"]`, '[Site "github.com/bkthomps/chess-ai"]'];
  if (game.started_at != null) {
    const date = new Date(game.started_at * 1000);
    if (!Number.isNaN(date.getTime())) tags.push(`[Date "${date.getFullYear()}.${String(date.getMonth() + 1).padStart(2, '0')}.${String(date.getDate()).padStart(2, '0')}"]`);
  }
  tags.push(`[White "${player(true)}"]`, `[Black "${player(false)}"]`, `[Result "${result}"]`);
  const moves = [];
  for (let i = 0; i < game.notation.length; i += 2) {
    moves.push(`${i / 2 + 1}. ${game.notation[i]}${game.notation[i + 1] ? ` ${game.notation[i + 1]}` : ''}`);
  }
  return `${tags.join('\n')}\n\n${[...moves, result].join(' ')}\n`;
}
$('copy-pgn').onclick = async () => {
  if (!state) return;
  clearTimeout(copyPgnTimer);
  $('copy-pgn').classList.remove('copied');
  try {
    await navigator.clipboard.writeText(gamePgn(state));
    $('copy-pgn').classList.add('copied');
    copyPgnTimer = setTimeout(() => $('copy-pgn').classList.remove('copied'), 1500);
  } catch {
    showError(new Error('Could not copy PGN. Check clipboard permissions.'));
  }
};
$('undo').onclick = async () => {
  if (busy) return; busy = true; $('undo').disabled = true; selected = null;
  clearBoardInput();
  try { state = await api('undo', {}); resetSoundSnapshot(state); }
  catch (error) { showError(error); }
  finally { busy = false; render(); }
};
$('resign').onclick = () => {
  if (busy || state.result || state.ai_vs_ai) return;
  clearBoardInput(); renderBoard();
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
  if (!busy && !polling && !document.hidden) {
    polling = true;
    try {
      const next = await api('state');
      if (document.body.classList.contains('disconnected')) {
        clearBoardInput(); cancelSounds(); soundSnapshot = null;
        state = null; playing = false; flipped = false; focusSquare = 12;
        lastHistoryLength = -1; lastStatus = '';
        $('board').replaceChildren(); $('history').replaceChildren();
        $('menu').hidden = false; $('game').hidden = true; resetNewGameConfirmation();
        $('error').hidden = true; $('engine-info').open = false;
        document.body.classList.remove('in-game');
        document.body.classList.remove('disconnected'); $('disconnected').hidden = true; syncMusic();
        $('start').focus({preventScroll:true});
      } else if (!playing || !state) { state = next; }
      // Never overwrite a newer move response with an older polling response.
      else if (!busy && (next.revision > state.revision || next.revision === state.revision && (next.thinking !== state.thinking || next.pondering !== state.pondering))) {
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
  setTimeout(poll, playing && (state?.thinking || state?.pondering) ? 150 : playing && state?.clocks.running ? 250 : 1000);
}
void poll();
