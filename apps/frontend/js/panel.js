const API = 'http://localhost:3000';
const PAK = new URLSearchParams(location.search).get('pak') || '';
let entries = [];

const keyBadge = document.getElementById('keyBadge');

function setKeyBadge(state) {
  keyBadge.className = `key-badge ${state}`;
  keyBadge.textContent = {
    ok: '🔑 ключ ок',
    missing: '🔑 нет ключа',
    bad: '🔑 ключ неверный',
  }[state];
  keyBadge.classList.remove('hidden');
}

function authFetch(input, options = {}) {
  const opts = { ...options };
  if (PAK) {
    opts.headers = { ...opts.headers, 'Authorization': `Bearer ${PAK}` };
  }
  return fetch(input, opts).then(res => {
    if (res.status === 401) setKeyBadge(PAK ? 'bad' : 'missing');
    else setKeyBadge('ok');
    return res;
  });
}

async function loadAll() {
  try {
    const [listRes, statsRes] = await Promise.all([
      authFetch(`${API}/api/queue`),
      authFetch(`${API}/api/queue/stats`),
    ]);
    if (listRes.ok) entries = await listRes.json();

    const hasSpinning = entries.some(e => e.status === 'Spinning');
    if (!hasSpinning) {
      const next = entries.find(e => e.status === 'Error' || e.status === 'Pending');
      document.getElementById('nextUser').textContent = next ? (next.user_name || '') : '';
    }

    if (statsRes.ok) {
      const s = await statsRes.json();
      document.getElementById('btnNext').textContent = `▶ Dequeue (${s.pending + s.error})`;
    }
    renderTables();
  } catch { /* */ }
}

function renderTables() {
  const active = entries.filter(e => e.status === 'Pending' || e.status === 'Spinning' || e.status === 'Error');
  const done = entries.filter(e => e.status === 'Completed' || e.status === 'Cancelled');

  renderActive(active);
  renderDone(done);
}

function renderActive(items) {
  const tbody = document.getElementById('tblActive');
  if (items.length === 0) {
    tbody.innerHTML = '<tr><td colspan="4" class="empty-row">Нет записей</td></tr>';
    return;
  }
  tbody.innerHTML = items.map(e => {
    const badge = `<span class="status-badge ${e.status.toLowerCase()}">${e.status}</span>`;
    let actions = '';
    if (e.status === 'Pending') {
      actions = `<button class="btn-cancel" onclick="cancelEntry(${e.id})">✕</button>`;
    } else if (e.status === 'Spinning') {
      actions = `<button class="btn-complete" onclick="completeEntry(${e.id})">✔</button>`;
    } else if (e.status === 'Error') {
      actions = `<button class="btn-cancel" onclick="cancelEntry(${e.id})">✕</button>`;
    }
    const slotInfo = e.status === 'Spinning' ? (e.slot_name || e.result_slot_id || '—') : '—';
    return `<tr>
      <td>${e.user_name || e.user_id}</td>
      <td>${badge}</td>
      <td>${slotInfo}</td>
      <td class="actions-cell">${actions}</td>
    </tr>`;
  }).join('');
}

function renderDone(items) {
  const tbody = document.getElementById('tblDone');
  if (items.length === 0) {
    tbody.innerHTML = '<tr><td colspan="3" class="empty-row">Нет записей</td></tr>';
    return;
  }
  tbody.innerHTML = [...items].reverse().map(e => {
    const badge = `<span class="status-badge ${e.status.toLowerCase()}">${e.status}</span>`;
    const result = e.status === 'Completed' ? (e.slot_name || '✔') : 'отменен';
    return `<tr>
      <td>${e.user_name || e.user_id}</td>
      <td>${result}</td>
      <td>${badge}</td>
    </tr>`;
  }).join('');
}

async function dequeueNext() {
  document.getElementById('btnNext').disabled = true;
  try {
    const r = await authFetch(`${API}/api/queue/next`, { method: 'POST' });
    const data = await r.json();
    if (r.ok) {
      addEvent(`🎰 ${data.slot.name} → #${data.entry.id} (${data.entry.user_name})`, 'start');
    } else {
      addEvent(`❌ Dequeue: ${r.status}`, 'error');
    }
    await loadAll();
  } catch (err) {
    addEvent(`❌ ${err.message}`, 'error');
  } finally {
    document.getElementById('btnNext').disabled = false;
  }
}

async function completeEntry(id) {
  try {
    const r = await authFetch(`${API}/api/queue/${id}/complete`, { method: 'POST' });
    if (r.ok) addEvent(`✔ #${id} завершён`, 'complete');
    else addEvent(`❌ Complete #${id}: ${r.status}`, 'error');
    await loadAll();
  } catch (err) {
    addEvent(`❌ ${err.message}`, 'error');
  }
}

async function cancelEntry(id) {
  try {
    const r = await authFetch(`${API}/api/queue/${id}/cancel`, { method: 'POST' });
    if (r.ok) addEvent(`✕ #${id} отменён`, 'error');
    else addEvent(`❌ Cancel #${id}: ${r.status}`, 'error');
    await loadAll();
  } catch (err) {
    addEvent(`❌ ${err.message}`, 'error');
  }
}

function addEvent(msg, type) {
  const log = document.getElementById('eventLog');
  const div = document.createElement('div');
  div.className = `ev ev-${type}`;
  div.innerHTML = `<span class="ev-time">[${new Date().toLocaleTimeString()}]</span>${msg}`;
  log.prepend(div);
  while (log.children.length > 50) log.removeChild(log.lastChild);
}

async function enqueueEntry() {
  const name = document.getElementById('enqName').value.trim();
  if (!name) return;
  document.getElementById('btnEnqueue').disabled = true;
  try {
    const r = await authFetch(`${API}/api/queue/anonymous`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ name }),
    });
    if (r.ok) {
      document.getElementById('enqName').value = '';
      addEvent(`➕ ${name} добавлен`, 'complete');
    } else {
      addEvent(`❌ Ошибка ${r.status}`, 'error');
    }
    await loadAll();
  } catch (err) {
    addEvent(`❌ ${err.message}`, 'error');
  } finally {
    document.getElementById('btnEnqueue').disabled = false;
  }
}

document.getElementById('btnNext').addEventListener('click', dequeueNext);
document.getElementById('btnRefresh').addEventListener('click', loadAll);
document.getElementById('btnEnqueue').addEventListener('click', enqueueEntry);
document.getElementById('btnToggleEnqueue').addEventListener('click', () => {
  const el = document.getElementById('enqueueSection');
  el.classList.toggle('hidden');
  document.getElementById('btnToggleEnqueue').textContent = el.classList.contains('hidden') ? '+ Добавить' : '✕ Закрыть';
});
document.getElementById('btnToggleLog').addEventListener('click', () => {
  const el = document.getElementById('logSection');
  el.classList.toggle('hidden');
  document.getElementById('btnToggleLog').textContent = el.classList.contains('hidden') ? '📋 Лог' : '✕ Лог';
});

loadAll();
setInterval(loadAll, 10000);

const connDot = document.getElementById('connDot');
let ws = null;

function connectWs() {
  ws = new WebSocket(`ws://localhost:3000/ws`);
  ws.onopen = () => { connDot.className = 'conn-dot connected'; };
  ws.onclose = () => {
    connDot.className = 'conn-dot disconnected';
    setTimeout(connectWs, 3000);
  };
  ws.onerror = () => { ws.close(); };
  ws.onmessage = e => {
    const d = JSON.parse(e.data);
    switch (d.type) {
      case 'spin_started':
        addEvent(`🎰 #${d.entry_id} — ${d.user_name}: ${d.slot_name}`, 'start');
        loadAll();
        break;
      case 'spin_completed':
        addEvent(`✔ #${d.entry_id} завершён`, 'complete');
        loadAll();
        break;
      case 'spin_error':
        addEvent(`⚠ #${d.entry_id} таймаут`, 'error');
        loadAll();
        break;
    }
  };
}

connectWs();
