const MESSAGES = {
  en: {
    minimize: 'Minimize',
    hideToTray: 'Hide to tray',
    input: 'Input',
    output: 'Output',
    toggleMute: 'Toggle mute',
    loading: 'Loading…',
    refresh: 'Refresh',
    autoSwitch: 'Auto-switch',
    autoSwitchSub: 'Make ★ devices the default when plugged in',
    autostart: 'Start with Windows',
    autostartSub: 'Waits quietly in the system tray',
    currentInput: 'Current input',
    currentOutput: 'Current output',
    inputDevices: 'Input devices',
    outputDevices: 'Output devices',
    noInput: 'No microphone found',
    noOutput: 'No output device found',
    connectDevice: 'Connect a device',
    muted: 'Muted',
    defaultDevice: 'Default device',
    clickToSwitch: 'Click to switch',
    starOn: 'Switch automatically when plugged in',
    starOff: 'Stop switching automatically',
    starredToast: '★ Will switch automatically when plugged in',
    unstarredToast: 'Removed from auto-switch',
    switchedTo: (name) => `Switched to ${name}`,
    language: 'Language',
    langAuto: 'Automatic (Windows)',
    settings: 'Settings',
    close: 'Close',
  },
  ja: {
    minimize: '最小化',
    hideToTray: 'トレイに隠す',
    input: '入力',
    output: '出力',
    toggleMute: 'ミュート切り替え',
    loading: '読み込み中…',
    refresh: '再読み込み',
    autoSwitch: '自動切り替え',
    autoSwitchSub: '★ のデバイスを接続したら既定にする',
    autostart: 'Windows 起動時に常駐',
    autostartSub: 'タスクトレイで待機します',
    currentInput: '現在の入力',
    currentOutput: '現在の出力',
    inputDevices: '入力デバイス',
    outputDevices: '出力デバイス',
    noInput: 'マイクが見つかりません',
    noOutput: '出力デバイスが見つかりません',
    connectDevice: 'デバイスを接続してください',
    muted: 'ミュート中',
    defaultDevice: '既定のデバイス',
    clickToSwitch: 'クリックで切り替え',
    starOn: '接続したら自動で切り替える',
    starOff: '自動切り替えの対象から外す',
    starredToast: '★ 接続すると自動で切り替わります',
    unstarredToast: '自動切り替えの対象から外しました',
    switchedTo: (name) => `${name} に切り替わりました`,
    language: '言語',
    langAuto: '自動（Windows に合わせる）',
    settings: '詳細設定',
    close: '閉じる',
  },
};

const systemLang = () =>
  (navigator.languages?.[0] || navigator.language || 'en').toLowerCase().startsWith('ja') ? 'ja' : 'en';

let langPref = (() => {
  try {
    return localStorage.getItem('lang') || 'auto';
  } catch {
    return 'auto';
  }
})();
let LANG = langPref === 'auto' ? systemLang() : langPref;

const t = (key, ...args) => {
  const value = MESSAGES[LANG][key] ?? MESSAGES.en[key];
  return typeof value === 'function' ? value(...args) : value;
};

function applyI18n() {
  document.documentElement.lang = LANG;
  document.querySelectorAll('[data-i18n]').forEach((el) => (el.textContent = t(el.dataset.i18n)));
  document.querySelectorAll('[data-i18n-title]').forEach((el) => (el.title = t(el.dataset.i18nTitle)));
}

function setLangPref(pref) {
  const before = LANG;
  langPref = ['ja', 'en'].includes(pref) ? pref : 'auto';
  LANG = langPref === 'auto' ? systemLang() : langPref;
  try {
    localStorage.setItem('lang', langPref);
  } catch {}
  applyI18n();
  return before !== LANG;
}

applyI18n();
