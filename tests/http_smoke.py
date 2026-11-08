"""Run after `cargo build --release`: python3 tests/http_smoke.py."""
import http.client
import json
import pathlib
import subprocess
import time
import urllib.parse

root = pathlib.Path(__file__).resolve().parents[1]
server = subprocess.Popen([str(root / 'target/release/chess-ai'), '--port', '0'],
                          stdout=subprocess.PIPE, text=True)
try:
    for line in server.stdout:
        if 'http://' in line:
            port = int(line.split('http://127.0.0.1:')[1].split('/')[0].split()[0])
            break
    else:
        raise AssertionError('Server did not start')

    def request(path, params=None, expected=200):
        conn = http.client.HTTPConnection('127.0.0.1', port, timeout=8)
        headers = {'X-Chess-Request': '1', 'Origin': f'http://127.0.0.1:{port}'}
        conn.request('GET' if params is None else 'POST', path +
                     (('?' + urllib.parse.urlencode(params)) if params else ''), headers=headers)
        response = conn.getresponse()
        body = response.read().decode()
        assert response.status == expected, (response.status, body)
        conn.close()
        return json.loads(body) if path.startswith('/api/') else body

    def new(mode='two', color='white', base_seconds=60, increment_seconds=0):
        return request('/api/new', dict(mode=mode, color=color, base_seconds=base_seconds, increment_seconds=increment_seconds))

    def move(state, uci, expected=200):
        def sq(s): return ord(s[0]) - 97 + (int(s[1]) - 1) * 8
        return request(
                       '/api/move', {'from': sq(uci[:2]), 'to': sq(uci[2:4]),
                       'promotion': {'q': 5, 'r': 4, 'b': 3, 'n': 2}.get(uci[4:], 0),
                       'revision': state['revision']}, expected)

    def next_analysis(state, nodes=0):
        deadline = time.monotonic() + 4
        while time.monotonic() < deadline:
            updated = request('/api/state')
            assert updated['revision'] == state['revision']
            analysis = updated['analysis']
            assert analysis['revision'] == updated['revision'] and analysis['active']
            if analysis['nodes'] > nodes and analysis['millis'] > 0:
                return updated
            time.sleep(.025)
        raise AssertionError('Live analysis did not advance')

    html = request('/')
    assert '<title>Chess</title>' in html and 'Time control' in html
    state = new()
    assert len(state['legal']) == 20 and state['human'] is None
    assert state['clocks']['running'] and state['clocks']['base_seconds'] == 60
    old = state
    state = move(state, 'e2e4')
    assert state['notation'] == ['e4'] and not state['white']
    move(old, 'd2d4', 400)
    move(state, 'e7e4', 400)
    state = request('/api/undo', {})
    assert not state['notation'] and state['white']
    for uci in ['f2f3', 'e7e5', 'g2g4', 'd8h4']:
        state = move(state, uci)
    assert state['result'] == 'Black wins by checkmate' and not state['legal']
    assert state['notation'][-1] == 'Qh4#'
    state = request('/api/undo', {})
    assert state['result'] is None
    state = request('/api/resign', {})
    assert state['result'] == 'White wins by resignation'
    state = new('one', 'white')
    assert state['pondering'] and not state['thinking']
    pondered = next_analysis(state)
    pondered = next_analysis(pondered, pondered['analysis']['nodes'])
    assert not pondered['notation'] and pondered['revision'] == state['revision']
    assert pondered['clocks']['black_ms'] == state['clocks']['black_ms']
    assert pondered['analysis']['budget_millis'] is None and pondered['analysis']['pondering']
    assert pondered['analysis']['threads'] >= 1
    assert 0 < pondered['analysis']['cache_bytes'] <= pondered['analysis']['cache_limit'] < 100_000_000
    assert pondered['analysis']['searching_depth'] > pondered['analysis']['depth']
    state = move(state, 'e2e4')
    state = next_analysis(state)
    assert state['thinking'] and not state['analysis']['pondering']
    assert state['analysis']['budget_millis'] > 0
    deadline = time.monotonic() + 8
    while state['thinking'] and time.monotonic() < deadline:
        time.sleep(.05)
        state = request('/api/state')
    assert len(state['notation']) == 2 and state['white'] and state['stats']['nodes'] > 0
    assert state['stats']['threads'] >= 1
    assert 0 < state['stats']['cache_bytes'] <= state['stats']['cache_limit'] < 100_000_000
    assert state['pondering'] and not state['thinking']
    state = request('/api/undo', {})
    assert not state['notation'] and state['white']
    assert state['pondering']
    state = new('one', 'black')
    deadline = time.monotonic() + 8
    while state['thinking'] and time.monotonic() < deadline:
        time.sleep(.05)
        state = request('/api/state')
    assert len(state['notation']) == 1 and not state['white']
    # Replacing a game while a worker searches must invalidate that result.
    state = new('one', 'black', 5)
    state = new('two')
    time.sleep(.2)
    state = request('/api/state')
    assert state['human'] is None and not state['notation'] and not state['thinking']
    assert not state['pondering']
    state = request('/api/new', dict(mode='two', color='white', unlimited='true'))
    assert not state['clocks']['enabled'] and not state['clocks']['running']
    state = move(state, 'e2e4')
    state = request('/api/undo', {})
    assert not state['clocks']['running'] and state['result'] is None
    state = request('/api/new', dict(mode='one', color='black', unlimited='true', thinking_seconds=0.1))
    deadline = time.monotonic() + 5
    while state['thinking'] and time.monotonic() < deadline:
        time.sleep(.02)
        state = request('/api/state')
    assert len(state['notation']) == 1 and state['stats']['budget_millis'] == 100
    assert not state['clocks']['enabled'] and not state['clocks']['running']
    assert state['pondering']
    state = request('/api/resign', {})
    assert not state['pondering'] and not state['thinking']
    assert not state['analysis']['active']
    print('HTTP integration passed: local play, illegal/stale moves, mate, undo, resignation, both AI sides, pondering, worker isolation.')
finally:
    server.terminate()
    server.wait(timeout=5)
