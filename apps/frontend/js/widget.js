const API = 'http://localhost:3000';
let currentEntryId = null;
let idleTimer = null;
const AUTO_MS = 10000;

const $ = id => document.getElementById(id);
const idleText = $('idleText');
const spinInfo = $('spinInfo');
const userName = $('userName');
const slotName = $('slotName');
const slotRarity = $('slotRarity');
const entryId = $('entryId');
const stateLabel = $('stateLabel');
const spinner = $('spinner');
const sseDot = $('sseDot');
const sseLabel = $('sseLabel');

function setIdle() {
  clearTimeout(idleTimer);
  stateLabel.textContent = 'Ожидание';
  idleText.classList.remove('hidden');
  spinInfo.classList.add('hidden');
  spinner.classList.add('hidden');
}

function setSpinning(data) {
  clearTimeout(idleTimer);
  currentEntryId = data.entry_id;
  stateLabel.textContent = 'Крутится!';
  idleText.classList.add('hidden');
  spinInfo.classList.remove('hidden');
  spinner.classList.remove('hidden');
  userName.textContent = data.user_name;
  slotName.textContent = data.slot_name;
  slotRarity.textContent = data.slot_rarity;
  entryId.textContent = `#${data.entry_id}`;
  idleTimer = setTimeout(() => {
    if (currentEntryId === data.entry_id) {
      fetch(`${API}/api/queue/${currentEntryId}/complete`, { method: 'POST' }).catch(() => {});
      setCompleted();
    }
  }, AUTO_MS);
}

function setCompleted() {
  clearTimeout(idleTimer);
  stateLabel.textContent = '✔ Завершён';
  spinner.classList.add('hidden');
  idleTimer = setTimeout(setIdle, 4000);
}

function setError(data) {
  clearTimeout(idleTimer);
  stateLabel.textContent = '⚠ Ошибка';
  spinner.classList.add('hidden');
  if (data) entryId.textContent = `#${data.entry_id} — таймаут`;
  idleTimer = setTimeout(setIdle, 4000);
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
