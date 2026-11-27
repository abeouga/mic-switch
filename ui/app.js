const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;
const appWindow = window.__TAURI__.window.getCurrentWindow();

const $ = (selector) => document.querySelector(selector);

const ICONS = {
  mic: '<svg viewBox="0 0 24 24"><rect class="fill" x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7"/></svg>',
  micOff: '<svg viewBox="0 0 24 24"><rect x="9" y="3" width="6" height="11" rx="3"/><path d="M5.5 11a6.5 6.5 0 0 0 13 0M12 17.5V21M8.5 21h7M4 4l16 16"/></svg>',
  speaker: '<svg viewBox="0 0 24 24"><path class="fill" d="M4 9v6h4l5 4V5L8 9z"/><path d="M16.5 8.5a5 5 0 0 1 0 7M19 6a8.5 8.5 0 0 1 0 12"/></svg>',
  speakerOff: '<svg viewBox="0 0 24 24"><path d="M4 9v6h4l5 4V5L8 9z"/><path d="M17 9.5l5 5M22 9.5l-5 5"/></svg>',
  star: '<svg viewBox="0 0 24 24"><path d="M12 3.6l2.6 5.2 5.8.9-4.2 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.2-4.1 5.8-.9z"/></svg>',
};

const tabMeta = (kind) =>
  kind === 'output'
    ? { label: t('currentOutput'), listTitle: t('outputDevices'), icon: ICONS.speaker, iconOff: ICONS.speakerOff, empty: t('noOutput') }
    : { label: t('currentInput'), listTitle: t('inputDevices'), icon: ICONS.mic, iconOff: ICONS.micOff, empty: t('noInput') };

let state = null;
let tab = loadTab();
const lastDefaultIds = { input: null, output: null };
let draggingVolume = false;
let volumeTimer = null;
let toastTimer = null;

function loadTab() {
  try {
    return localStorage.getItem('tab') === 'output' ? 'output' : 'input';
  } catch {
    return 'input';
  }
}

const escapeHtml = (s) =>
  s.replace(/[&<>"']/g, (c) => ({ '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' })[c]);

function splitName(name) {
  const m = name.match(/^(.*?)\s*\((.+)\)$/);
  return m ? { main: m[2], sub: m[1] } : { main: name, sub: '' };
}

function toast(message, isError = false) {
  const el = $('#toast');
  el.textContent = message;
  el.classList.toggle('error', isError);
  el.classList.add('show');
  clearTimeout(toastTimer);
  toastTimer = setTimeout(() => el.classList.remove('show'), 2600);
}

function setVolumeUi(level) {
  const pct = Math.round(level * 100);
  const slider = $('#vol');
  slider.value = pct;
  slider.style.setProperty('--p', pct + '%');
  $('#volValue').textContent = pct;
}

const currentDevice = () => state?.[tab].find((d) => d.isDefault);

function notifyExternalChanges(next, switchedByUser) {
  for (const kind of ['input', 'output']) {
    const current = next[kind].find((d) => d.isDefault);
    const prev = lastDefaultIds[kind];
    if (current && prev && current.id !== prev && kind !== switchedByUser) toast(t('switchedTo', splitName(current.name).main));
    lastDefaultIds[kind] = current?.id ?? null;
  }
}

function render() {
  if (!state) return;
  const meta = tabMeta(tab);
  const devices = state[tab];
  const current = currentDevice();

  $('#tabs').dataset.tab = tab;
  $('#heroLabel').textContent = meta.label;
  $('#listTitle').textContent = meta.listTitle;

  const hero = $('#hero');
  hero.classList.toggle('muted', !!current?.muted);
  hero.classList.toggle('empty', !current);
  $('#muteBtn').innerHTML = current?.muted ? meta.iconOff : meta.icon;
  if (current) {
    const n = splitName(current.name);
    $('#current').textContent = n.main;
    $('#currentSub').textContent = current.muted ? t('muted') : n.sub;
  } else {
    $('#current').textContent = meta.empty;
    $('#currentSub').textContent = t('connectDevice');
  }
  $('#vol').disabled = !current;
  if (!draggingVolume) setVolumeUi(current?.volume ?? 0);

  const listHtml =
    devices
      .map((d, i) => {
        const n = splitName(d.name);
        const sub = d.isDefault ? t('defaultDevice') : n.sub || t('clickToSwitch');
        return `
          <div class="dev glass ${d.isDefault ? 'active' : ''}" data-id="${escapeHtml(d.id)}" style="animation-delay:${i * 30}ms">
            <div class="dev-icon">${meta.icon}</div>
            <div class="dev-text"><b>${escapeHtml(n.main)}</b><small>${escapeHtml(sub)}</small></div>
            <button class="star ${d.starred ? 'on' : ''}" data-name="${escapeHtml(d.name)}"
              title="${d.starred ? t('starOff') : t('starOn')}">${ICONS.star}</button>
          </div>`;
      })
      .join('') || `<div class="empty glass">${meta.empty}</div>`;
  const list = $('#list');
  if (listHtml !== list.dataset.html) {
    list.innerHTML = listHtml;
    list.dataset.html = listHtml;
  }

  $('#autoSwitch').checked = state.autoSwitch;
  $('#autostart').checked = state.autostart;
  document.querySelectorAll('#langMenu [data-lang]').forEach((el) => el.setAttribute('aria-checked', String(el.dataset.lang === langPref)));
}

async function call(command, args, switchedByUser = null) {
  try {
    const result = await invoke(command, args);
    if (result) {
      notifyExternalChanges(result, switchedByUser);
      state = result;
      if (result.language !== langPref) setLangPref(result.language);
      render();
    }
    return result;
  } catch (e) {
    toast(String(e), true);
  }
}

const refresh = () => call('get_state');

$('#tabs').addEventListener('click', (e) => {
  const btn = e.target.closest('button');
  if (!btn || btn.dataset.tab === tab) return;
  tab = btn.dataset.tab;
  try {
    localStorage.setItem('tab', tab);
  } catch {}
  playEnter();
  render();
});

function playEnter() {
  const list = $('#list');
  list.classList.remove('enter');
  void list.offsetWidth;
  list.classList.add('enter');
  clearTimeout(playEnter.timer);
  playEnter.timer = setTimeout(() => list.classList.remove('enter'), 800);
}

$('#list').addEventListener('click', async (e) => {
  const star = e.target.closest('.star');
  if (star) {
    const result = await call('toggle_star', { name: star.dataset.name });
    const starred = result?.[tab].find((d) => d.name === star.dataset.name)?.starred;
    if (result) toast(starred ? t('starredToast') : t('unstarredToast'));
    return;
  }
  const card = e.target.closest('.dev');
  if (card && !card.classList.contains('active')) await call('set_default', { id: card.dataset.id }, tab);
});

$('#muteBtn').addEventListener('click', () => {
  const current = currentDevice();
  if (current) call('set_mute', { id: current.id, muted: !current.muted });
});

const slider = $('#vol');
slider.addEventListener('pointerdown', () => (draggingVolume = true));
for (const type of ['pointerup', 'pointercancel']) window.addEventListener(type, () => (draggingVolume = false));
slider.addEventListener('change', () => {
  draggingVolume = false;
  refresh();
});
slider.addEventListener('input', () => {
  const level = slider.value / 100;
  setVolumeUi(level);
  const current = currentDevice();
  if (!current) return;
  clearTimeout(volumeTimer);
  volumeTimer = setTimeout(() => invoke('set_volume', { id: current.id, level }).catch((e) => toast(String(e), true)), 40);
});

$('#autoSwitch').addEventListener('change', (e) => call('set_auto_switch', { enabled: e.target.checked }));
$('#autostart').addEventListener('change', (e) => call('set_autostart', { enabled: e.target.checked }));

$('#refreshBtn').addEventListener('click', (e) => {
  const btn = e.currentTarget;
  btn.classList.remove('spin');
  void btn.offsetWidth;
  btn.classList.add('spin');
  refresh();
});

const langMenu = $('#langMenu');
const closeLangMenu = () => (langMenu.hidden = true);

$('#langBtn').addEventListener('click', (e) => {
  e.stopPropagation();
  langMenu.hidden = !langMenu.hidden;
});
langMenu.addEventListener('click', (e) => {
  const item = e.target.closest('[data-lang]');
  if (!item) return;
  closeLangMenu();
  setLangPref(item.dataset.lang);
  render();
  call('set_language', { language: item.dataset.lang });
});
document.addEventListener('click', (e) => {
  if (!langMenu.hidden && !langMenu.contains(e.target)) closeLangMenu();
});

const sheet = $('#settingsSheet');
const setSheetOpen = (open) => {
  sheet.classList.toggle('open', open);
  sheet.setAttribute('aria-hidden', String(!open));
};
$('#settingsBtn').addEventListener('click', () => setSheetOpen(!sheet.classList.contains('open')));
$('#settingsClose').addEventListener('click', () => setSheetOpen(false));
sheet.querySelector('.sheet-backdrop').addEventListener('click', () => setSheetOpen(false));

document.addEventListener('keydown', (e) => {
  if (e.key !== 'Escape') return;
  closeLangMenu();
  setSheetOpen(false);
});

$('#minBtn').addEventListener('click', () => appWindow.minimize());
$('#closeBtn').addEventListener('click', () => appWindow.hide());

document.addEventListener('contextmenu', (e) => e.preventDefault());

listen('devices-changed', refresh);
window.addEventListener('focus', refresh);
$('#tabs').dataset.tab = tab;
playEnter();
refresh();
