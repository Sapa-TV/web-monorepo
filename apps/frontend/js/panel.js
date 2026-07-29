const API = 'http://localhost:3000';
let entries = [];

async function loadAll() {
  try {
    const [listRes, statsRes] = await Promise.all([
      fetch(`${API}/api/queue`),
      fetch(`${API}/api/queue/stats`),
    ]);
    if (listRes.ok) entries = await listRes.json();
    if (statsRes.ok) {
      const s = await statsRes.json();
      document.getElementById('cntPending').textContent = s.pending ?? 0;
      document.getElementById('cntSpinning').textContent = s.spinning ?? 0;
      document.getElementById('cntCompleted').textContent = s.completed ?? 0;
      document.getElementById('cntError').textContent = s.error ?? 0;
      document.getElementById('cntCancelled').textContent = s.cancelled ?? 0;
    }
    renderTables();
  } catch { /* ignore */ }
}

function renderTables() {
  const pending = entries.filter(e => e.status === 'Pending');
  const spinning = entries.filter(e => e.status === 'Spinning');
  const done = entries.filter(e => e.status !== 'Pending' && e.status !== 'Spinning');

  renderTable('tblPending', pending, false);
  renderTable('tblSpinning', spinning, true);
  renderTable('tblDone', done, false);
}

function renderTable(id, items) {
  const tbody = document.getElementById(id);
  if (items.length === 0) {
    tbody.innerHTML = '<tr><td colspan="6" class="empty">Нет записей</td></tr>';
    return;
  }
  tbody.innerHTML = items.map(e => {
    const badge = `<span class="status-badge ${e.status.toLowerCase()}">${e.status}</span>`;
    let actions = '';
    if (e.status === 'Pending') {
      actions = `<button class="btn-cancel" onclick="cancelEntry(${e.id})">Отменить</button>`;
    } else if (e.status === 'Spinning') {
      actions = `<button class="btn-complete" onclick="completeEntry(${e.id})">✔ Завершить</button>`;
    } else if (e.status === 'Error') {
      actions = `<button class="btn-cancel" onclick="cancelEntry(${e.id})">Отменить</button>`;
    }
    return `<tr>
      <td>${e.id}</td>
      <td>${e.user_id}</td>
      <td>${badge}</td>
      <td>${e.result_slot_id ?? '—'}</td>
      <td>${new Date(e.updated_at).toLocaleTimeString()}</td>
      <td class="actions-cell">${actions}</td>
    </tr>`;
  }).join('');
}

async function dequeueNext() {
  document.getElementById('btnNext').disabled = true;
  try {
    const r = await fetch(`${API}/api/queue/next`, { method: 'POST' });
    const data = await r.json();
    if (r.ok) {
      addEvent(`🎰 ${data.slot.name} выпал для #${data.entry.id}`, 'start');
      await loadAll();
    } else {
      addEvent(`❌ Dequeue: ${r.status} ${data.message || JSON.stringify(data)}`, 'error');
    }
  } catch (err) {
    addEvent(`❌ Ошибка: ${err.message}`, 'error');
  } finally {
    document.getElementById('btnNext').disabled = false;
  }
}

async function completeEntry(id) {
  try {
    const r = await fetch(`${API}/api/queue/${id}/complete`, { method: 'POST' });
    if (r.ok) {
      addEvent(`✔ #${id} завершён`, 'complete');
      await loadAll();
    } else {
      addEvent(`❌ Complete #${id}: ${r.status}`, 'error');
    }
  } catch (err) {
    addEvent(`❌ Ошибка: ${err.message}`, 'error');
  }
}

async function cancelEntry(id) {
  try {
    const r = await fetch(`${API}/api/queue/${id}/cancel`, { method: 'POST' });
    if (r.ok) {
      addEvent(`✕ #${id} отменён`, 'error');
      await loadAll();
    } else {
      addEvent(`❌ Cancel #${id}: ${r.status}`, 'error');
    }
  } catch (err) {
    addEvent(`❌ Ошибка: ${err.message}`, 'error');
  }
}

function addEvent(msg, type) {
  const log = document.getElementById('eventLog');
  const div = document.createElement('div');
  div.className = `ev ev-${type}`;
  const time = new Date().toLocaleTimeString();
  div.innerHTML = `<span class="ev-time">[${time}]</span>${msg}`;
  log.prepend(div);
  while (log.children.length > 50) log.removeChild(log.lastChild);
}

async function enqueueEntry() {
  const platform = document.getElementById('enqPlatform').value;
  const userId = document.getElementById('enqUserId').value.trim();
  const username = document.getElementById('enqUsername').value.trim();
  if (!userId || !username) return;
  document.getElementById('btnEnqueue').disabled = true;
  try {
    const r = await fetch(`${API}/api/queue`, {
      method: 'POST',
      headers: { 'Content-Type': 'application/json' },
      body: JSON.stringify({ platform, platform_user_id: userId, platform_username: username }),
    });
    if (r.ok) {
      document.getElementById('enqUserId').value = '';
      document.getElementById('enqUsername').value = '';
      addEvent(`➕ #${(await r.json()).id} добавлен`, 'complete');
      await loadAll();
    } else {
      addEvent(`❌ Ошибка ${r.status}`, 'error');
    }
  } catch (err) {
    addEvent(`❌ ${err.message}`, 'error');
  } finally {
    document.getElementById('btnEnqueue').disabled = false;
  }
}

document.getElementById('btnNext').addEventListener('click', dequeueNext);
document.getElementById('btnRefresh').addEventListener('click', loadAll);
document.getElementById('btnEnqueue').addEventListener('click', enqueueEntry);

loadAll();
setInterval(loadAll, 10000);

const evtSource = new EventSource(`${API}/api/events`);
const sseDot = document.getElementById('sseDot');
const sseStatus = document.getElementById('sseStatus');

evtSource.onopen = () => {
  sseDot.className = 'sse-dot connected';
  sseStatus.textContent = 'SSE connected';
};
evtSource.onerror = () => {
  sseDot.className = 'sse-dot disconnected';
  sseStatus.textContent = 'SSE disconnected';
};
evtSource.addEventListener('spin_started', e => {
  const d = JSON.parse(e.data);
  addEvent(`🎰 Спин #${d.entry_id} — ${d.user_name}: ${d.slot_name} (${d.slot_rarity})`, 'start');
  loadAll();
});
evtSource.addEventListener('spin_completed', e => {
  const d = JSON.parse(e.data);
  addEvent(`✔ Спин #${d.entry_id} завершён`, 'complete');
  loadAll();
});
evtSource.addEventListener('spin_error', e => {
  const d = JSON.parse(e.data);
  addEvent(`⚠ Спин #${d.entry_id} таймаут`, 'error');
  loadAll();
});