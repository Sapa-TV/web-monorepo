const API = 'http://localhost:3000';
let currentEntryId = null;
let autoTimer = null;
const AUTO_MS = 10000;

const $ = id => document.getElementById(id);
const idleText = $('idleText');
const spinInfo = $('spinInfo');
const userName = $('userName');
const slotName = $('slotName');
const slotRarity = $('slotRarity');
const entryId = $('entryId');
const autoTimerEl = $('autoTimer');
const stateLabel = $('stateLabel');
const spinner = $('spinner');
const sseDot = $('sseDot');
const sseLabel = $('sseLabel');

function setIdle() {
  clearTimeout(autoTimer);
  stateLabel.textContent = 'Ожидание';
  idleText.classList.remove('hidden');
  spinInfo.classList.add('hidden');
  autoTimerEl.classList.add('hidden');
  spinner.classList.add('hidden');
}

function startAutoConfirm() {
  const start = Date.now();
  const fill = autoTimerEl.querySelector('.bar-fill');
  autoTimerEl.classList.remove('hidden');

  function tick() {
    const remaining = Math.max(0, AUTO_MS - (Date.now() - start));
    fill.style.width = (remaining / AUTO_MS) * 100 + '%';
    if (remaining > 0) requestAnimationFrame(tick);
  }
  requestAnimationFrame(tick);

  autoTimer = setTimeout(async () => {
    if (!currentEntryId) return;
    try { await fetch(`${API}/api/queue/${currentEntryId}/complete`, { method: 'POST' }); } catch { /* */ }
    setCompleted();
  }, AUTO_MS);
}

function setSpinning(data) {
  clearTimeout(autoTimer);
  currentEntryId = data.entry_id;
  stateLabel.textContent = 'Крутится!';
  idleText.classList.add('hidden');
  spinInfo.classList.remove('hidden');
  spinner.classList.remove('hidden');
  userName.textContent = data.user_name;
  slotName.textContent = data.slot_name;
  slotRarity.textContent = data.slot_rarity;
  entryId.textContent = `#${data.entry_id}`;
  startAutoConfirm();
}

function setCompleted() {
  clearTimeout(autoTimer);
  stateLabel.textContent = '✔ Завершён';
  spinner.classList.add('hidden');
  autoTimerEl.classList.add('hidden');
  setTimeout(setIdle, 4000);
}

function setError(data) {
  clearTimeout(autoTimer);
  stateLabel.textContent = '⚠ Ошибка';
  spinner.classList.add('hidden');
  autoTimerEl.classList.add('hidden');
  if (data) entryId.textContent = `#${data.entry_id} — таймаут`;
  setTimeout(setIdle, 4000);
}

let ws = null;

function connectWs() {
  ws = new WebSocket(`ws://localhost:3000/ws`);

  ws.onopen = () => {
    sseDot.className = 'sse-dot connected';
    sseLabel.textContent = 'подключено';
  };

  ws.onmessage = e => {
    const d = JSON.parse(e.data);
    switch (d.type) {
      case 'spin_started':
        setSpinning(d);
        break;
      case 'spin_completed':
        if (currentEntryId === d.entry_id) setCompleted();
        break;
      case 'spin_error':
        if (currentEntryId === d.entry_id) setError(d);
        break;
    }
  };

  ws.onclose = () => {
    sseDot.className = 'sse-dot disconnected';
    sseLabel.textContent = 'отключено';
    setTimeout(connectWs, 3000);
  };

  ws.onerror = () => {
    ws.close();
  };
}

connectWs();
setIdle();
