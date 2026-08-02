const API = 'http://localhost:3000';
const PAK = new URLSearchParams(location.search).get('pak') || '';
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
const connDot = $('connDot');
const connLabel = $('connLabel');
const keyBadge = $('keyBadge');

let keyOk = false;

function setConn(state, label) {
  connDot.className = `conn-dot ${state}`;
  connLabel.textContent = label;
}

function setKeyBadge(state, text) {
  keyBadge.className = `key-badge ${state}`;
  keyBadge.textContent = text;
  keyBadge.classList.remove('hidden');
}

function failAuth() {
  if (ws) {
    ws.onclose = null;
    ws.close();
    ws = null;
  }
  clearTimeout(idleTimer);
  setConn('disconnected', 'не авторизован');
  setKeyBadge(PAK ? 'bad' : 'missing', PAK ? 'ключ неверный' : 'нет ключа');
  stateLabel.textContent = 'Доступ запрещён';
  idleText.textContent = 'Ошибка подключения виджета.';
  spinner.classList.add('hidden');
  spinInfo.classList.add('hidden');
}

function authFetch(input, options = {}) {
  const opts = { ...options };
  if (PAK) {
    opts.headers = { ...opts.headers, 'Authorization': `Bearer ${PAK}` };
  }
  return fetch(input, opts);
}

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
      authFetch(`${API}/api/queue/${currentEntryId}/complete`, { method: 'POST' }).catch(() => {});
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
let wsQueued = false;

function connectWs() {
  if (!keyOk || wsQueued) return;
  wsQueued = true;
  ws = new WebSocket(`ws://localhost:3000/ws`);

  ws.onopen = () => {
    setConn('connected', 'подключено');
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
    wsQueued = false;
    setConn('disconnected', 'отключено');
    if (keyOk) setTimeout(connectWs, 3000);
  };

  ws.onerror = () => {
    ws.close();
  };
}

if (!PAK) {
  failAuth();
} else {
  setIdle();
  authFetch(`${API}/api/queue`)
    .then(res => {
      if (res.status === 401) {
        failAuth();
      } else if (!res.ok) {
        setKeyBadge('bad', 'ошибка запроса');
        stateLabel.textContent = 'Ошибка подключения виджета';
        idleText.textContent = 'Ошибка подключения виджета.';
      } else {
        keyOk = true;
        connectWs();
      }
    })
    .catch(() => {
      setConn('disconnected', 'нет связи');
      setKeyBadge('bad', 'нет связи с сервером');
      stateLabel.textContent = 'Нет связи';
      idleText.textContent = 'Сервер недоступен.';
    });
}
