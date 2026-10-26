const ICONS = {
  menu: '<path d="M4 6h16M4 12h16M4 18h16"/>',
  search: '<circle cx="11" cy="11" r="7"/><path d="m20 20-3.5-3.5"/>',
  refresh: '<path d="M20 12a8 8 0 1 1-2.34-5.66"/><path d="M20 4v5h-5"/>',
  settings: '<path d="M4 6h9M17 6h3M4 12h3M11 12h9M4 18h11M19 18h1"/><circle cx="15" cy="6" r="2"/><circle cx="9" cy="12" r="2"/><circle cx="17" cy="18" r="2"/>',
  plus: '<path d="M12 5v14M5 12h14"/>',
  star: '<path d="m12 3.5 2.6 5.3 5.9.9-4.3 4.1 1 5.8L12 16.9l-5.2 2.7 1-5.8-4.3-4.1 5.9-.9z"/>',
  check: '<path d="m5 12.5 4.5 4.5L19 7.5"/>',
  checkAll: '<path d="m2.5 12.5 4.5 4.5 9.5-9.5"/><path d="m12.5 16 1 1 9-9.5"/>',
  list: '<path d="M9 6h12M9 12h12M9 18h12M4 6h.01M4 12h.01M4 18h.01"/>',
  cards: '<rect x="3.5" y="4" width="17" height="7" rx="1.5"/><rect x="3.5" y="14" width="17" height="6" rx="1.5"/>',
  external: '<path d="M14 4h6v6M20 4l-9 9"/><path d="M18 14v5a1 1 0 0 1-1 1H5a1 1 0 0 1-1-1V7a1 1 0 0 1 1-1h5"/>',
  chevronDown: '<path d="m6 9 6 6 6-6"/>',
  more: '<circle cx="5" cy="12" r="1.3"/><circle cx="12" cy="12" r="1.3"/><circle cx="19" cy="12" r="1.3"/>',
  folder: '<path d="M3.5 7.5a2 2 0 0 1 2-2h3.8l2 2h7.2a2 2 0 0 1 2 2v7.5a2 2 0 0 1-2 2h-13a2 2 0 0 1-2-2z"/>',
  inbox: '<path d="M3.5 13h4.8l1.7 2.5h4l1.7-2.5h4.8"/><path d="M5.6 5h12.8l2.1 8v5a1 1 0 0 1-1 1h-15a1 1 0 0 1-1-1v-5z"/>',
  x: '<path d="M6 6l12 12M18 6 6 18"/>',
  filter: '<path d="M4 5h16l-6.2 7.5V18l-3.6 2v-7.5z"/>',
  sort: '<path d="M7 4v16M3.5 16.5 7 20l3.5-3.5M17 20V4M13.5 7.5 17 4l3.5 3.5"/>',
  logout: '<path d="M14 4h4a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2h-4M10 16l-4-4 4-4M6 12h10"/>',
  download: '<path d="M12 4v11M7 10.5l5 5 5-5M4.5 20h15"/>',
  upload: '<path d="M12 16V5M7 9.5l5-5 5 5M4.5 20h15"/>',
  keyboard: '<rect x="2.5" y="6" width="19" height="12" rx="2"/><path d="M6.5 10h.01M10.5 10h.01M14.5 10h.01M18 10h.01M7.5 14h9"/>',
  rss: '<path d="M5 11a8 8 0 0 1 8 8M5 5a14 14 0 0 1 14 14"/><circle cx="6" cy="18" r="1.2"/>',
  alert: '<path d="M12 4 2.8 19.5h18.4z"/><path d="M12 10v4M12 17h.01"/>',
  eyeOff: '<path d="M3 3l18 18M10.6 6.1A9.8 9.8 0 0 1 12 6c5 0 8.5 4.2 9.5 6-.5.9-1.6 2.5-3.2 3.8M6.5 7.6C4.6 8.9 3.2 10.8 2.5 12c1 1.8 4.5 6 9.5 6 1.5 0 2.9-.4 4.1-1"/><path d="M9.9 9.9a3 3 0 0 0 4.2 4.2"/>',
  edit: '<path d="M4 20h4L19 9l-4-4L4 16z"/><path d="m13.5 6.5 4 4"/>',
  trash: '<path d="M4 7h16M9 7V4.5h6V7M6.5 7l1 13h9l1-13"/>',
  share: '<path d="M12 4v11M7.5 8.5 12 4l4.5 4.5"/><path d="M5 13v6a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-6"/>',
  sun: '<circle cx="12" cy="12" r="4"/><path d="M12 2.5v2M12 19.5v2M2.5 12h2M19.5 12h2M5.3 5.3l1.4 1.4M17.3 17.3l1.4 1.4M5.3 18.7l1.4-1.4M17.3 6.7l1.4-1.4"/>',
  circle: '<circle cx="12" cy="12" r="4.5"/>',
  heart: '<path d="M12 20s-7.5-4.6-7.5-10.2A4.2 4.2 0 0 1 12 7.4a4.2 4.2 0 0 1 7.5 2.4C19.5 15.4 12 20 12 20z"/>',
  butterfly: '<path d="M12 11c-1.6-3.2-5.4-7-8-7-1.5 0-1.5 2.6-.8 5.2.7 2.5 2.6 3.3 4.8 3.1-3.3.7-3.8 3.2-1.6 5.3C9 20 11 17 12 14c1 3 3 6 5.6 3.6 2.2-2.1 1.7-4.6-1.6-5.3 2.2.2 4.1-.6 4.8-3.1.7-2.6.7-5.2-.8-5.2-2.6 0-6.4 3.8-8 7z"/>',
};

const $ = (sel, root = document) => root.querySelector(sel);

function icon(name, cls = '') {
  const ns = 'http://www.w3.org/2000/svg';
  const svg = document.createElementNS(ns, 'svg');
  svg.setAttribute('viewBox', '0 0 24 24');
  svg.setAttribute('class', `i ${cls}`.trim());
  svg.setAttribute('aria-hidden', 'true');
  svg.innerHTML = ICONS[name];
  return svg;
}

function h(tag, attrs, ...children) {
  const el = document.createElement(tag);
  for (const [k, v] of Object.entries(attrs || {})) {
    if (v == null || v === false) continue;
    if (k === 'class') el.className = v;
    else if (k === 'style') el.style.cssText = v;
    else if (k === 'dataset') Object.assign(el.dataset, v);
    else if (k.startsWith('on')) el.addEventListener(k.slice(2), v);
    else if (k === 'html') el.innerHTML = v;
    else if (v === true) el.setAttribute(k, '');
    else el.setAttribute(k, v);
  }
  for (const c of children.flat(Infinity)) {
    if (c == null || c === false) continue;
    el.append(c instanceof Node ? c : document.createTextNode(String(c)));
  }
  return el;
}

/* ---------------- preferences ---------------- */

const PREF_KEY = 'yesrss.prefs';
const DEFAULT_PREFS = {
  view: 'list', unreadOnly: true, oldest: false, theme: 'auto', density: 'normal',
  markOnScroll: true, collapsed: {}, sidebarUnreadOnly: false,
};

function loadPrefs() {
  try {
    return { ...DEFAULT_PREFS, ...JSON.parse(localStorage.getItem(PREF_KEY) || '{}') };
  } catch {
    return { ...DEFAULT_PREFS };
  }
}

function savePrefs() {
  try { localStorage.setItem(PREF_KEY, JSON.stringify(S.prefs)); } catch { /* storage unavailable */ }
  applyPrefs();
}

function applyPrefs() {
  const root = document.documentElement;
  if (S.prefs.theme === 'auto') delete root.dataset.theme; else root.dataset.theme = S.prefs.theme;
  root.dataset.density = S.prefs.density;
}

/* ---------------- state ---------------- */

const S = {
  user: null,
  tree: { folders: [], feeds: [], unread: 0, starred: 0 },
  route: { kind: 'all' },
  entries: [],
  byId: new Map(),
  next: null,
  asOf: 0,
  loading: false,
  done: false,
  selected: -1,
  openId: null,
  gen: 0,
  prefs: loadPrefs(),
  atproto: { enabled: false, account: null },
};

/* ---------------- api ---------------- */

class ApiError extends Error {
  constructor(status, message) { super(message); this.status = status; }
}

async function api(method, path, body) {
  const opts = { method, headers: { 'X-Yesrss': '1' }, credentials: 'same-origin' };
  if (body !== undefined) {
    if (typeof body === 'string') {
      opts.body = body;
      opts.headers['Content-Type'] = 'text/xml';
    } else {
      opts.body = JSON.stringify(body);
      opts.headers['Content-Type'] = 'application/json';
    }
  }
  const res = await fetch(`/api${path}`, opts);
  if (res.status === 401 && S.user && !path.startsWith('/login')) {
    S.user = null;
    renderLogin();
    throw new ApiError(401, 'Your session has expired. Please sign in again.');
  }
  if (!res.ok) {
    let msg = res.statusText;
    try { msg = (await res.json()).error || msg; } catch { /* not json */ }
    throw new ApiError(res.status, msg);
  }
  if (res.status === 204 || res.status === 202) return null;
  const ct = res.headers.get('content-type') || '';
  return ct.includes('json') ? res.json() : res.text();
}

function toast(msg, kind = '') {
  const el = h('div', { class: `toast ${kind}` }, msg);
  $('#toasts').append(el);
  setTimeout(() => el.remove(), kind === 'error' ? 6000 : 3000);
}

const fail = (e) => toast(e.message || String(e), 'error');

/* ---------------- formatting ---------------- */

function relTime(sec) {
  const d = new Date(sec * 1000);
  const now = new Date();
  const diff = (now - d) / 1000;
  if (diff < 60) return 'now';
  if (diff < 3600) return `${Math.floor(diff / 60)}m`;
  if (diff < 86400 && d.getDate() === now.getDate()) {
    return d.toLocaleTimeString(undefined, { hour: 'numeric', minute: '2-digit' });
  }
  if (diff < 6 * 86400) return d.toLocaleDateString(undefined, { weekday: 'short' });
  if (d.getFullYear() === now.getFullYear()) {
    return d.toLocaleDateString(undefined, { month: 'short', day: 'numeric' });
  }
  return d.toLocaleDateString(undefined, { year: 'numeric', month: 'short', day: 'numeric' });
}

const fullTime = (sec) => new Date(sec * 1000).toLocaleString(undefined, { dateStyle: 'full', timeStyle: 'short' });

function agoLong(sec) {
  if (!sec) return 'never';
  const diff = Date.now() / 1000 - sec;
  if (diff < 90) return 'just now';
  if (diff < 3600) return `${Math.round(diff / 60)} min ago`;
  if (diff < 86400) return `${Math.round(diff / 3600)} h ago`;
  return `${Math.round(diff / 86400)} d ago`;
}

function hue(str) {
  let x = 0;
  for (const c of str) x = (x * 31 + c.charCodeAt(0)) >>> 0;
  return x % 360;
}

function feedIcon(feed) {
  const title = feed?.title || '?';
  const avatar = () => h('span', { class: 'avatar', style: `background: hsl(${hue(title)} 45% 50%)` }, [...title.replace(/^the\s+/i, '')][0] || '?');
  if (!feed?.has_icon) return avatar();
  const img = h('img', { class: 'favicon', src: `/api/feeds/${feed.id}/icon`, alt: '', loading: 'lazy' });
  img.addEventListener('error', () => img.replaceWith(avatar()), { once: true });
  return img;
}

const feedById = (id) => S.tree.feeds.find((f) => f.id === id);
const folderById = (id) => S.tree.folders.find((f) => f.id === id);

/* ---------------- routing ---------------- */

function parseRoute() {
  const parts = location.hash.replace(/^#\/?/, '').split('/');
  switch (parts[0]) {
    case 'starred': return { kind: 'starred' };
    case 'feed': return { kind: 'feed', id: Number(parts[1]) };
    case 'folder': return { kind: 'folder', id: Number(parts[1]) };
    case 'search': return { kind: 'search', q: decodeURIComponent(parts.slice(1).join('/')) };
    default: return { kind: 'all' };
  }
}

function routeHash(r) {
  switch (r.kind) {
    case 'starred': return '#/starred';
    case 'feed': return `#/feed/${r.id}`;
    case 'folder': return `#/folder/${r.id}`;
    case 'search': return `#/search/${encodeURIComponent(r.q)}`;
    default: return '#/all';
  }
}

const go = (r) => {
  const hash = routeHash(r);
  if (location.hash === hash) loadStream(); else location.hash = hash;
};

function routeTitle(r) {
  switch (r.kind) {
    case 'starred': return 'Starred';
    case 'feed': return feedById(r.id)?.title || 'Feed';
    case 'folder': return folderById(r.id)?.name || 'Folder';
    case 'search': return `Search: ${r.q}`;
    default: return 'All items';
  }
}

/* ---------------- auth screens ---------------- */

function authCard(title, subtitle, submitLabel, onSubmit, extra) {
  const err = h('div', { class: 'error-text' });
  const user = h('input', { class: 'input', id: 'u', name: 'username', autocomplete: 'username', required: true, autocapitalize: 'none', spellcheck: 'false' });
  const pass = h('input', { class: 'input', id: 'p', name: 'password', type: 'password', required: true, autocomplete: extra ? 'new-password' : 'current-password' });
  const btn = h('button', { class: 'btn primary', type: 'submit' }, submitLabel);
  const form = h('form', {
    onsubmit: async (ev) => {
      ev.preventDefault();
      btn.disabled = true;
      err.textContent = '';
      try { await onSubmit(user.value.trim(), pass.value); } catch (e) { err.textContent = e.message; } finally { btn.disabled = false; }
    },
  },
  h('div', { class: 'field' }, h('label', { for: 'u' }, 'Username'), user),
  h('div', { class: 'field' }, h('label', { for: 'p' }, 'Password'), pass),
  extra && h('div', { class: 'hint' }, extra),
  err, btn);
  $('#app').replaceChildren(h('div', { class: 'auth' },
    h('div', { class: 'auth-card' },
      h('span', { class: 'brand' }, h('img', { src: 'icon.svg', alt: '' }), h('span', {}, 'yes', h('b', {}, 'rss'))),
      h('h1', {}, title), h('p', {}, subtitle), form)));
  user.focus();
}

function renderLogin() {
  document.title = 'Sign in · yesrss';
  authCard('Welcome back', 'Sign in to your reader.', 'Sign in', async (username, password) => {
    const r = await api('POST', '/login', { username, password });
    S.user = r.user;
    boot();
  });
}

function renderSetup() {
  document.title = 'Set up · yesrss';
  authCard('Create your account', 'This is a fresh install. The first account you create owns it.', 'Create account', async (username, password) => {
    const r = await api('POST', '/setup', { username, password });
    S.user = r.user;
    boot();
  }, 'At least 8 characters. Use the same credentials in mobile apps via the Google Reader API.');
}

/* ---------------- shell ---------------- */

function renderShell() {
  const search = h('input', {
    type: 'search', placeholder: 'Search all items', 'aria-label': 'Search', id: 'search',
    onkeydown: (ev) => {
      if (ev.key === 'Enter' && search.value.trim()) { go({ kind: 'search', q: search.value.trim() }); search.blur(); }
      if (ev.key === 'Escape') { search.value = ''; search.blur(); }
    },
  });
  const shell = h('div', { class: 'shell' },
    h('header', { class: 'topbar' },
      h('button', { class: 'btn ghost icon menu-btn', 'aria-label': 'Menu', onclick: () => shell.classList.toggle('drawer') }, icon('menu')),
      h('a', { class: 'brand', href: '#/all' }, h('img', { src: 'icon.svg', alt: '' }), h('span', {}, 'yes', h('b', {}, 'rss'))),
      h('div', { class: 'search' }, icon('search'), search),
      h('div', { class: 'spacer' }),
      h('button', { class: 'btn ghost icon', title: 'Refresh all feeds (Shift+R)', 'aria-label': 'Refresh all feeds', onclick: refreshAll, id: 'refresh-btn' }, icon('refresh')),
      h('button', { class: 'btn ghost icon', title: 'Settings', 'aria-label': 'Settings', onclick: () => openSettings() }, icon('settings'))),
    h('nav', { class: 'sidebar', id: 'sidebar', 'aria-label': 'Subscriptions' }),
    h('div', { class: 'scrim', onclick: () => shell.classList.remove('drawer') }),
    h('main', { class: 'main', id: 'main' }));
  $('#app').replaceChildren(shell);
  $('#main').addEventListener('scroll', onMainScroll, { passive: true });
}

const closeDrawer = () => $('.shell')?.classList.remove('drawer');

/* ---------------- sidebar ---------------- */

function navItem({ label, count, active, href, iconEl, onMenu, warn, title }) {
  return h('a', {
    class: `nav-item${active ? ' active' : ''}${count ? ' has-unread' : ''}`, href, title: title || label,
    onclick: closeDrawer,
  },
  iconEl,
  h('span', { class: 'label' }, label),
  warn && h('span', { class: 'warn', title: warn }, icon('alert', 'sm')),
  count ? h('span', { class: 'count' }, count > 999 ? '999+' : count) : null,
  onMenu && h('button', {
    class: 'btn ghost icon more', 'aria-label': `Options for ${label}`,
    onclick: (ev) => { ev.preventDefault(); ev.stopPropagation(); onMenu(ev.currentTarget); },
  }, icon('more', 'sm')));
}

function renderSidebar() {
  const nav = $('#sidebar');
  if (!nav) return;
  const r = S.route;
  const t = S.tree;
  const hideRead = S.prefs.sidebarUnreadOnly;
  const feedNode = (f) => navItem({
    label: f.title, count: f.unread, href: `#/feed/${f.id}`, iconEl: feedIcon(f),
    active: r.kind === 'feed' && r.id === f.id,
    warn: f.error_count > 2 && f.last_error ? `Failing: ${f.last_error}` : null,
    onMenu: (anchor) => feedMenu(anchor, f),
  });
  const visible = (f) => !hideRead || f.unread > 0 || (r.kind === 'feed' && r.id === f.id);

  const children = [
    h('button', { class: 'btn primary subscribe', onclick: () => openSubscribe() }, icon('plus'), 'Subscribe'),
    navItem({ label: 'All items', count: t.unread, href: '#/all', iconEl: icon('inbox'), active: r.kind === 'all' }),
    navItem({ label: 'Starred', count: t.starred, href: '#/starred', iconEl: icon('star'), active: r.kind === 'starred' }),
    h('div', { class: 'nav-section' }, h('span', {}, 'Subscriptions'),
      h('button', {
        class: 'folder-toggle', title: hideRead ? 'Show all feeds' : 'Hide feeds with no unread items',
        'aria-label': 'Toggle feeds without unread items',
        onclick: () => { S.prefs.sidebarUnreadOnly = !hideRead; savePrefs(); renderSidebar(); },
      }, icon(hideRead ? 'eyeOff' : 'filter', 'sm'))),
  ];

  for (const folder of t.folders) {
    const feeds = t.feeds.filter((f) => f.folder_id === folder.id);
    const unread = feeds.reduce((a, f) => a + f.unread, 0);
    const shown = feeds.filter(visible);
    if (hideRead && !unread && !(r.kind === 'folder' && r.id === folder.id)) continue;
    const collapsed = !!S.prefs.collapsed[folder.id];
    const toggle = h('button', {
      class: 'folder-toggle', 'aria-label': collapsed ? 'Expand folder' : 'Collapse folder', 'aria-expanded': String(!collapsed),
      onclick: (ev) => {
        ev.preventDefault(); ev.stopPropagation();
        S.prefs.collapsed[folder.id] = !S.prefs.collapsed[folder.id];
        if (!S.prefs.collapsed[folder.id]) delete S.prefs.collapsed[folder.id];
        savePrefs(); renderSidebar();
      },
    }, icon('chevronDown', 'sm'));
    children.push(h('div', { class: `folder${collapsed ? ' collapsed' : ''}` },
      navItem({
        label: folder.name, count: unread, href: `#/folder/${folder.id}`, iconEl: toggle,
        active: r.kind === 'folder' && r.id === folder.id, onMenu: (a) => folderMenu(a, folder, unread),
      }),
      h('div', { class: 'folder-feeds' }, shown.map(feedNode))));
  }
  const loose = t.feeds.filter((f) => f.folder_id == null);
  children.push(...loose.filter(visible).map(feedNode));
  if (!t.feeds.length) {
    children.push(h('div', { class: 'empty-subs' }, 'No subscriptions yet. Add one, or import an OPML file from Settings.'));
  }
  nav.replaceChildren(...children);
  document.title = t.unread ? `(${t.unread}) yesrss` : 'yesrss';
}

async function loadTree() {
  S.tree = await api('GET', '/tree');
  renderSidebar();
}

function adjustUnread(entry, delta) {
  const f = feedById(entry.feed_id);
  if (f) f.unread = Math.max(0, f.unread + delta);
  S.tree.unread = Math.max(0, S.tree.unread + delta);
  renderSidebar();
}

/* ---------------- popup menus ---------------- */

function popupMenu(anchor, items) {
  document.querySelector('.menu')?.remove();
  const menu = h('div', { class: 'menu', role: 'menu' },
    items.map((it) => (it === '-' ? h('hr') : h('button', {
      role: 'menuitem', class: it.danger ? 'danger' : '',
      onclick: () => { menu.remove(); it.action(); },
    }, icon(it.icon, 'sm'), it.label))));
  document.body.append(menu);
  const r = anchor.getBoundingClientRect();
  const mw = menu.offsetWidth;
  const mh = menu.offsetHeight;
  menu.style.left = `${Math.max(8, Math.min(r.right - mw, innerWidth - mw - 8))}px`;
  menu.style.top = `${r.bottom + mh + 8 > innerHeight ? Math.max(8, r.top - mh - 4) : r.bottom + 4}px`;
  menu.querySelector('button')?.focus();
  const close = (ev) => {
    if (ev.type === 'keydown' && ev.key !== 'Escape') return;
    if (ev.type === 'pointerdown' && menu.contains(ev.target)) return;
    menu.remove();
    document.removeEventListener('pointerdown', close, true);
    document.removeEventListener('keydown', close, true);
  };
  setTimeout(() => {
    document.addEventListener('pointerdown', close, true);
    document.addEventListener('keydown', close, true);
  });
}

function feedMenu(anchor, f) {
  popupMenu(anchor, [
    { label: 'Mark all as read', icon: 'checkAll', action: () => markAllRead({ feed: f.id }, f.title, f.unread) },
    { label: 'Refresh now', icon: 'refresh', action: () => refreshFeed(f) },
    { label: 'Rename…', icon: 'edit', action: () => renameFeed(f) },
    { label: 'Move to folder…', icon: 'folder', action: () => moveFeed(f) },
    f.site_url && { label: 'Open website', icon: 'external', action: () => window.open(f.site_url, '_blank', 'noopener,noreferrer') },
    '-',
    { label: 'Unsubscribe', icon: 'trash', danger: true, action: () => unsubscribe(f) },
  ].filter(Boolean));
}

function folderMenu(anchor, folder, unread) {
  popupMenu(anchor, [
    { label: 'Mark all as read', icon: 'checkAll', action: () => markAllRead({ folder: folder.id }, folder.name, unread) },
    { label: 'Rename…', icon: 'edit', action: () => renameFolder(folder) },
    '-',
    { label: 'Delete folder', icon: 'trash', danger: true, action: () => deleteFolder(folder) },
  ]);
}

/* ---------------- dialogs ---------------- */

function dialog({ title, body, actions, wide, onClose }) {
  const dlg = h('dialog', { class: wide ? 'wide' : '' },
    h('div', { class: 'dlg-head' }, h('h3', {}, title),
      h('button', { class: 'btn ghost icon', 'aria-label': 'Close', onclick: () => dlg.close() }, icon('x'))),
    body,
    actions && h('div', { class: 'dlg-foot' }, actions));
  dlg.addEventListener('close', () => { dlg.remove(); onClose?.(); });
  document.body.append(dlg);
  dlg.showModal();
  return dlg;
}

function ask({ title, message, confirm = 'OK', danger = false, input = null }) {
  return new Promise((resolve) => {
    let result = null;
    const field = input && h('input', { class: 'input', value: input.value || '', placeholder: input.placeholder || '' });
    const form = h('form', {
      method: 'dialog',
      onsubmit: (ev) => { ev.preventDefault(); result = field ? field.value : true; dlg.close(); },
    },
    h('div', { class: 'dlg-body' }, message && h('p', { style: 'margin:0 0 12px' }, message), field),
    h('div', { class: 'dlg-foot' },
      h('button', { class: 'btn', type: 'button', onclick: () => dlg.close() }, 'Cancel'),
      h('button', { class: `btn ${danger ? 'danger' : 'primary'}`, type: 'submit' }, confirm)));
    const dlg = dialog({ title, body: form, onClose: () => resolve(result) });
    if (field) { field.focus(); field.select(); }
  });
}

/* ---------------- feed / folder actions ---------------- */

async function markAllRead(scope, label, count) {
  if (count > 25) {
    const ok = await ask({ title: 'Mark all as read?', message: `Mark ${count} items in “${label}” as read?`, confirm: 'Mark as read' });
    if (!ok) return;
  }
  try {
    const sameAsView = (scope.feed && S.route.kind === 'feed' && S.route.id === scope.feed)
      || (scope.folder && S.route.kind === 'folder' && S.route.id === scope.folder)
      || (!scope.feed && !scope.folder && !scope.starred && S.route.kind === 'all');
    await api('POST', '/entries/mark-read', { ...scope, as_of: sameAsView && S.asOf ? S.asOf : undefined });
    await loadTree();
    for (const e of S.entries) {
      const hit = (!scope.feed && !scope.folder && !scope.starred)
        || (scope.feed && e.feed_id === scope.feed)
        || (scope.folder && feedById(e.feed_id)?.folder_id === scope.folder)
        || (scope.starred && e.is_starred);
      if (hit && !e.is_read) { e.is_read = true; updateEntryEl(e); }
    }
    toast('Marked as read');
  } catch (e) { fail(e); }
}

async function refreshFeed(f) {
  try {
    const r = await api('POST', `/feeds/${f.id}/refresh`);
    await loadTree();
    toast(r.new ? `${r.new} new item${r.new === 1 ? '' : 's'} in ${f.title}` : `${f.title} is up to date`);
    if (r.new && (S.route.kind === 'all' || (S.route.kind === 'feed' && S.route.id === f.id) || (S.route.kind === 'folder' && S.route.id === f.folder_id))) loadStream();
  } catch (e) { fail(e); await loadTree().catch(() => {}); }
}

async function refreshAll() {
  const btn = $('#refresh-btn svg');
  btn?.classList.add('spin');
  try {
    await api('POST', '/refresh');
    toast('Refreshing all feeds…');
    const before = S.tree.unread;
    for (let i = 0; i < 6; i++) {
      await new Promise((r) => setTimeout(r, 1500 + i * 1000));
      await loadTree();
    }
    if (S.tree.unread !== before) loadStream();
  } catch (e) { fail(e); } finally { btn?.classList.remove('spin'); }
}

async function renameFeed(f) {
  const title = await ask({ title: 'Rename subscription', confirm: 'Rename', input: { value: f.title } });
  if (title == null) return;
  try { await api('PATCH', `/feeds/${f.id}`, { title }); await loadTree(); renderStreamHead(); } catch (e) { fail(e); }
}

function folderSelect(current, { allowNew = true } = {}) {
  const sel = h('select', { class: 'input' },
    h('option', { value: '' }, '— No folder —'),
    S.tree.folders.map((fo) => h('option', { value: fo.id, selected: fo.id === current }, fo.name)),
    allowNew && h('option', { value: '__new' }, 'New folder…'));
  return sel;
}

async function resolveFolder(sel) {
  if (sel.value === '__new') {
    const name = await ask({ title: 'New folder', confirm: 'Create', input: { placeholder: 'Folder name' } });
    if (!name?.trim()) return undefined;
    const r = await api('POST', '/folders', { name });
    return r.id;
  }
  return sel.value ? Number(sel.value) : null;
}

async function moveFeed(f) {
  const sel = folderSelect(f.folder_id);
  const dlg = dialog({
    title: `Move “${f.title}”`,
    body: h('div', { class: 'dlg-body' }, h('div', { class: 'field' }, h('label', {}, 'Folder'), sel)),
    actions: [
      h('button', { class: 'btn', onclick: () => dlg.close() }, 'Cancel'),
      h('button', {
        class: 'btn primary',
        onclick: async () => {
          try {
            const folder_id = await resolveFolder(sel);
            if (folder_id === undefined) return;
            await api('PATCH', `/feeds/${f.id}`, { folder_id });
            dlg.close();
            await loadTree();
          } catch (e) { fail(e); }
        },
      }, 'Move'),
    ],
  });
}

async function unsubscribe(f) {
  const ok = await ask({ title: 'Unsubscribe?', message: `Unsubscribe from “${f.title}”? Its items, including starred ones, will be deleted.`, confirm: 'Unsubscribe', danger: true });
  if (!ok) return;
  try {
    await api('DELETE', `/feeds/${f.id}`);
    await loadTree();
    if (S.route.kind === 'feed' && S.route.id === f.id) go({ kind: 'all' }); else loadStream();
    toast(`Unsubscribed from ${f.title}`);
  } catch (e) { fail(e); }
}

async function renameFolder(folder) {
  const name = await ask({ title: 'Rename folder', confirm: 'Rename', input: { value: folder.name } });
  if (name == null) return;
  try { await api('PATCH', `/folders/${folder.id}`, { name }); await loadTree(); renderStreamHead(); } catch (e) { fail(e); }
}

async function deleteFolder(folder) {
  const ok = await ask({ title: 'Delete folder?', message: `Delete “${folder.name}”? Its feeds stay subscribed and move to the top level.`, confirm: 'Delete folder', danger: true });
  if (!ok) return;
  try {
    await api('DELETE', `/folders/${folder.id}`);
    await loadTree();
    if (S.route.kind === 'folder' && S.route.id === folder.id) go({ kind: 'all' });
  } catch (e) { fail(e); }
}

/* ---------------- subscribe dialog ---------------- */

function openSubscribe(prefill = '') {
  const url = h('input', { class: 'input', type: 'text', placeholder: 'https://example.com or a feed URL', value: prefill, required: true, autocapitalize: 'none', spellcheck: 'false', inputmode: 'url' });
  const sel = folderSelect(S.route.kind === 'folder' ? S.route.id : null);
  const err = h('div', { class: 'error-text' });
  const cands = h('div', { class: 'candidates hidden' });
  const btn = h('button', { class: 'btn primary', type: 'submit' }, 'Subscribe');
  let chosen = null;

  const form = h('form', {
    onsubmit: async (ev) => {
      ev.preventDefault();
      err.textContent = '';
      btn.disabled = true;
      const original = btn.textContent;
      try {
        let target = chosen;
        if (!target) {
          btn.textContent = 'Looking for feeds…';
          const { candidates } = await api('POST', '/discover', { url: url.value });
          if (candidates.length > 1) {
            chosen = candidates[0].url;
            cands.replaceChildren(h('div', { class: 'hint' }, 'This site has several feeds. Pick one:'),
              candidates.map((c, i) => h('label', { class: 'candidate' },
                h('input', { type: 'radio', name: 'cand', checked: i === 0, onchange: () => { chosen = c.url; } }),
                h('span', {}, c.title || 'Untitled feed', h('small', {}, c.url)))));
            cands.classList.remove('hidden');
            return;
          }
          target = candidates[0].url;
        }
        btn.textContent = 'Subscribing…';
        const folder_id = await resolveFolder(sel);
        if (folder_id === undefined) return;
        const { id } = await api('POST', '/feeds', { url: target, folder_id });
        dlg.close();
        await loadTree();
        go({ kind: 'feed', id });
        toast(`Subscribed to ${feedById(id)?.title || 'feed'}`);
      } catch (e) {
        err.textContent = e.message;
      } finally {
        btn.disabled = false;
        btn.textContent = original;
      }
    },
  },
  h('div', { class: 'dlg-body' },
    h('div', { class: 'field' }, h('label', {}, 'Website or feed URL'), url,
      h('div', { class: 'hint' }, 'Paste a feed, a blog, a YouTube channel or a Mastodon profile. yesrss finds the feed.')),
    cands,
    h('div', { class: 'field' }, h('label', {}, 'Folder'), sel),
    err),
  h('div', { class: 'dlg-foot' }, h('button', { class: 'btn', type: 'button', onclick: () => dlg.close() }, 'Cancel'), btn));
  url.addEventListener('input', () => { chosen = null; cands.classList.add('hidden'); });
  const dlg = dialog({ title: 'Add a subscription', body: form });
  url.focus();
}

/* ---------------- stream ---------------- */

function streamParams(cursor) {
  const r = S.route;
  const p = new URLSearchParams();
  if (r.kind === 'feed') p.set('feed', r.id);
  if (r.kind === 'folder') p.set('folder', r.id);
  if (r.kind === 'starred') p.set('starred', 'true');
  if (r.kind === 'search') p.set('q', r.q);
  if (S.prefs.unreadOnly && r.kind !== 'starred' && r.kind !== 'search') p.set('unread', 'true');
  if (S.prefs.oldest) p.set('oldest', 'true');
  if (cursor) p.set('cursor', cursor);
  p.set('limit', '40');
  return p;
}

function renderStreamHead() {
  const main = $('#main');
  if (!main) return;
  const r = S.route;
  const feed = r.kind === 'feed' ? feedById(r.id) : null;
  const canFilter = r.kind !== 'starred' && r.kind !== 'search';
  const titleEl = feed?.site_url
    ? h('a', { href: feed.site_url, target: '_blank', rel: 'noopener noreferrer', title: 'Open website' }, routeTitle(r))
    : routeTitle(r);
  const head = h('div', { class: 'stream-head', id: 'stream-head' },
    h('h2', {}, feed ? feedIcon(feed) : null, h('span', {}, titleEl)),
    h('div', { class: 'tools' },
      canFilter && h('div', { class: 'seg', role: 'group', 'aria-label': 'Filter' },
        h('button', { class: `btn${S.prefs.unreadOnly ? ' on' : ''}`, onclick: () => setPref('unreadOnly', true) }, 'Unread'),
        h('button', { class: `btn${!S.prefs.unreadOnly ? ' on' : ''}`, onclick: () => setPref('unreadOnly', false) }, 'All')),
      h('div', { class: 'seg', role: 'group', 'aria-label': 'View' },
        h('button', { class: `btn icon${S.prefs.view === 'list' ? ' on' : ''}`, title: 'List view (1)', 'aria-label': 'List view', onclick: () => setPref('view', 'list') }, icon('list')),
        h('button', { class: `btn icon${S.prefs.view === 'expanded' ? ' on' : ''}`, title: 'Expanded view (2)', 'aria-label': 'Expanded view', onclick: () => setPref('view', 'expanded') }, icon('cards'))),
      h('button', { class: 'btn icon', title: S.prefs.oldest ? 'Oldest first' : 'Newest first', 'aria-label': 'Toggle sort order', onclick: () => setPref('oldest', !S.prefs.oldest) }, icon('sort')),
      r.kind !== 'search' && h('button', { class: 'btn', title: 'Mark all as read (Shift+A)', onclick: markViewRead }, icon('checkAll'), h('span', { class: 'label-text' }, 'Mark all read'))));
  const old = $('#stream-head');
  if (old) old.replaceWith(head); else main.prepend(head);
  $('#feed-error')?.remove();
  if (feed?.last_error && feed.error_count > 0) {
    head.after(h('div', { class: 'feed-error', id: 'feed-error' }, icon('alert', 'sm'),
      h('span', {}, `Last refresh failed ${agoLong(feed.last_fetched_at)}: ${feed.last_error}`)));
  }
}

function setPref(key, value) {
  S.prefs[key] = value;
  savePrefs();
  loadStream();
}

function markViewRead() {
  const r = S.route;
  if (r.kind === 'feed') return markAllRead({ feed: r.id }, routeTitle(r), feedById(r.id)?.unread || 0);
  if (r.kind === 'folder') {
    const n = S.tree.feeds.filter((f) => f.folder_id === r.id).reduce((a, f) => a + f.unread, 0);
    return markAllRead({ folder: r.id }, routeTitle(r), n);
  }
  if (r.kind === 'starred') return markAllRead({ starred: true }, 'Starred', S.entries.filter((e) => !e.is_read).length);
  return markAllRead({}, 'All items', S.tree.unread);
}

async function loadStream() {
  const gen = ++S.gen;
  S.route = parseRoute();
  S.entries = [];
  S.byId.clear();
  S.next = null;
  S.done = false;
  S.selected = -1;
  S.openId = null;
  const main = $('#main');
  if (!main) return;
  const search = $('#search');
  if (search) search.value = S.route.kind === 'search' ? S.route.q : '';
  main.replaceChildren();
  renderStreamHead();
  renderSidebar();
  main.append(h('div', { class: `entries view-${S.prefs.view}`, id: 'entries' }), h('div', { class: 'sentinel', id: 'sentinel' }));
  main.scrollTop = 0;
  observer.disconnect();
  observer.observe($('#sentinel'));
  await loadMore(gen);
}

async function loadMore(gen = S.gen) {
  if (S.loading || S.done) return;
  S.loading = true;
  const list = $('#entries');
  const spinner = h('div', { class: 'loading-more' }, 'Loading…');
  list?.after(spinner);
  try {
    const r = await api('GET', `/entries?${streamParams(S.next)}`);
    if (gen !== S.gen) return;
    if (!S.entries.length) S.asOf = r.as_of;
    for (const e of r.entries) {
      if (S.byId.has(e.id)) continue;
      S.entries.push(e);
      S.byId.set(e.id, e);
      list.append(entryEl(e));
    }
    S.next = r.next;
    S.done = !r.next;
    if (S.done) renderEnd();
  } catch (e) {
    if (gen === S.gen) fail(e);
  } finally {
    spinner.remove();
    if (gen === S.gen) S.loading = false;
  }
}

function renderEnd() {
  const list = $('#entries');
  if (!list) return;
  if (!S.entries.length) {
    const r = S.route;
    let title = 'All caught up';
    let msg = 'No unread items here. Nice.';
    let action = h('button', { class: 'btn', onclick: () => setPref('unreadOnly', false) }, 'Show read items');
    if (r.kind === 'starred') { title = 'No starred items'; msg = 'Press s on any item to star it for later.'; action = null; }
    if (r.kind === 'search') { title = 'No matches'; msg = `Nothing matched “${r.q}”.`; action = null; }
    if (!S.prefs.unreadOnly && r.kind !== 'search' && r.kind !== 'starred') { title = 'Nothing here yet'; msg = 'Items appear after the next refresh.'; action = null; }
    const broken = r.kind === 'feed' && feedById(r.id)?.error_count > 0 && !feedById(r.id)?.last_fetched_ok;
    if (broken) { title = 'This feed isn’t loading'; msg = 'yesrss will keep retrying with backoff. Check the URL in Settings → Subscriptions.'; action = h('button', { class: 'btn', onclick: () => refreshFeed(feedById(r.id)) }, icon('refresh', 'sm'), 'Try again'); }
    if (!S.tree.feeds.length) {
      title = 'Welcome to yesrss';
      msg = 'Subscribe to your first feed, or import an OPML export from FreshRSS or any other reader.';
      action = h('div', { class: 'radio-row', style: 'justify-content:center' },
        h('button', { class: 'btn primary', onclick: () => openSubscribe() }, icon('plus'), 'Subscribe'),
        h('button', { class: 'btn', onclick: () => openSettings('import') }, icon('upload'), 'Import OPML'));
    }
    list.append(h('div', { class: 'empty' }, icon(r.kind === 'starred' ? 'star' : 'checkAll'), h('h3', {}, title), h('p', {}, msg), action));
  } else {
    list.append(h('div', { class: 'end-marker' }, 'You’ve reached the end.'));
  }
}

const observer = new IntersectionObserver((items) => {
  if (items.some((i) => i.isIntersecting)) loadMore();
}, { root: null, rootMargin: '800px 0px' });

/* ---------------- entries ---------------- */

function entryEl(e) {
  const el = h('div', { class: 'entry', dataset: { id: e.id } });
  el.addEventListener('error', (ev) => {
    if (ev.target.tagName === 'IMG' && ev.target.closest('.content')) ev.target.style.display = 'none';
  }, true);
  fillEntry(el, e);
  return el;
}

function fillEntry(el, e) {
  const feed = feedById(e.feed_id) || { id: e.feed_id, title: e.feed_title };
  el.classList.toggle('read', e.is_read);
  el.classList.toggle('starred', e.is_starred);
  if (S.prefs.view === 'expanded') {
    el.replaceChildren(articleEl(e, feed));
    return;
  }
  const row = h('div', { class: 'row', onclick: (ev) => { if (!ev.target.closest('.star')) toggleOpen(e.id); } },
    h('button', { class: 'star', 'aria-label': e.is_starred ? 'Unstar' : 'Star', onclick: () => toggleStar(e) }, icon('star', 'sm')),
    h('span', { class: 'src' }, feedIcon(feed), h('span', {}, feed.title)),
    h('span', { class: 'line' }, h('b', {}, e.title), e.snippet ? h('span', { class: 'snip' }, ` — ${e.snippet}`) : null),
    h('time', { datetime: new Date(e.published_at * 1000).toISOString(), title: fullTime(e.published_at) }, relTime(e.published_at)));
  el.replaceChildren(row);
  if (S.openId === e.id) {
    el.classList.add('open');
    el.append(articleEl(e, feed));
  } else {
    el.classList.remove('open');
  }
}

function articleEl(e, feed) {
  const enclosure = e.enclosure_url && h('div', { class: 'enclosure' },
    e.enclosure_type?.startsWith('audio/') ? h('audio', { controls: true, preload: 'none', src: e.enclosure_url })
      : e.enclosure_type?.startsWith('video/') ? h('video', { controls: true, preload: 'none', src: e.enclosure_url })
        : h('a', { href: e.enclosure_url, target: '_blank', rel: 'noopener noreferrer' }, 'Download attachment'));
  const share = navigator.share && h('button', {
    class: 'btn ghost', onclick: () => navigator.share({ title: e.title, url: e.url }).catch(() => {}),
  }, icon('share', 'sm'), 'Share');
  return h('article', { class: 'article' },
    h('header', {},
      h('h1', {}, e.url ? h('a', { href: e.url, target: '_blank', rel: 'noopener noreferrer' }, e.title) : e.title),
      h('div', { class: 'meta' },
        h('a', { class: 'feedlink', href: `#/feed/${e.feed_id}` }, feedIcon(feed), feed.title),
        e.author && h('span', { class: 'dot' }), e.author && h('span', {}, e.author),
        h('span', { class: 'dot' }),
        h('time', { title: fullTime(e.published_at) }, relTime(e.published_at)))),
    enclosure,
    h('div', { class: 'content', html: e.content }),
    h('footer', {},
      h('button', { class: `btn ghost${e.is_starred ? ' starred-on' : ''}`, onclick: () => toggleStar(e) }, icon('star', 'sm'), e.is_starred ? 'Starred' : 'Star'),
      h('button', { class: `btn ghost${!e.is_read ? ' on' : ''}`, onclick: () => setRead(e, !e.is_read) }, icon(e.is_read ? 'circle' : 'check', 'sm'), e.is_read ? 'Mark unread' : 'Mark read'),
      S.atproto.account && e.url && h('button', {
        class: `btn ghost${e.like_uri ? ' liked-on' : ''}`, title: 'Like on Bluesky / atproto (l)', onclick: () => toggleLike(e),
      }, icon('heart', 'sm'), e.like_uri ? 'Liked' : 'Like'),
      e.url && h('a', { class: 'btn ghost', href: e.url, target: '_blank', rel: 'noopener noreferrer' }, icon('external', 'sm'), 'Open original'),
      share));
}

function updateEntryEl(e) {
  const el = document.querySelector(`.entry[data-id="${e.id}"]`);
  if (!el) return;
  if (S.prefs.view === 'expanded') {
    el.classList.toggle('read', e.is_read);
    el.classList.toggle('starred', e.is_starred);
    const footer = el.querySelector('footer');
    const fresh = articleEl(e, feedById(e.feed_id) || { id: e.feed_id, title: e.feed_title }).querySelector('footer');
    footer?.replaceWith(fresh);
  } else {
    fillEntry(el, e);
    if (S.entries[S.selected]?.id === e.id) el.classList.add('selected');
  }
}

async function setRead(e, read) {
  if (e.is_read === read) return;
  e.is_read = read;
  updateEntryEl(e);
  adjustUnread(e, read ? -1 : 1);
  try {
    await api('PATCH', `/entries/${e.id}`, { is_read: read });
  } catch (err) {
    e.is_read = !read;
    updateEntryEl(e);
    adjustUnread(e, read ? 1 : -1);
    fail(err);
  }
}

async function toggleStar(e) {
  const starred = !e.is_starred;
  e.is_starred = starred;
  S.tree.starred += starred ? 1 : -1;
  updateEntryEl(e);
  renderSidebar();
  try {
    await api('PATCH', `/entries/${e.id}`, { is_starred: starred });
  } catch (err) {
    e.is_starred = !starred;
    S.tree.starred += starred ? -1 : 1;
    updateEntryEl(e);
    renderSidebar();
    fail(err);
  }
}

const liking = new Set();
async function toggleLike(e) {
  if (!S.atproto.account) { openSettings('bluesky'); return; }
  if (!e.url || liking.has(e.id)) return;
  liking.add(e.id);
  const was = e.like_uri;
  e.like_uri = was ? null : 'pending';
  updateEntryEl(e);
  try {
    if (was) {
      await api('DELETE', `/entries/${e.id}/like`);
      toast('Like removed');
    } else {
      const r = await api('POST', `/entries/${e.id}/like`);
      e.like_uri = r.like_uri;
      toast(r.document_uri ? 'Liked, and recommended on standard.site' : `Liked as @${S.atproto.account.handle}`);
    }
  } catch (err) {
    e.like_uri = was;
    fail(err);
  } finally {
    liking.delete(e.id);
    updateEntryEl(e);
  }
}

function select(index, { scroll = true } = {}) {
  if (index < 0 || index >= S.entries.length) return;
  document.querySelector('.entry.selected')?.classList.remove('selected');
  S.selected = index;
  const e = S.entries[index];
  const el = document.querySelector(`.entry[data-id="${e.id}"]`);
  el?.classList.add('selected');
  if (scroll && el) {
    const main = $('#main');
    const headH = $('#stream-head')?.offsetHeight || 0;
    const top = el.offsetTop - headH - 8;
    if (S.prefs.view === 'expanded' || S.openId === e.id) {
      main.scrollTo({ top });
    } else {
      const rect = el.getBoundingClientRect();
      const mrect = main.getBoundingClientRect();
      if (rect.top < mrect.top + headH || rect.bottom > mrect.bottom) el.scrollIntoView({ block: 'nearest' });
    }
  }
  if (index >= S.entries.length - 5) loadMore();
}

function toggleOpen(id, force) {
  const idx = S.entries.findIndex((x) => x.id === id);
  if (idx < 0) return;
  const e = S.entries[idx];
  const opening = force ?? S.openId !== id;
  const prev = S.openId;
  S.openId = opening ? id : null;
  if (prev && prev !== id) {
    const pe = S.byId.get(prev);
    if (pe) updateEntryEl(pe);
  }
  updateEntryEl(e);
  select(idx);
  if (opening && !e.is_read) setRead(e, true);
}

function move(delta, open) {
  if (!S.entries.length) return;
  const idx = S.selected < 0 ? 0 : Math.max(0, Math.min(S.entries.length - 1, S.selected + delta));
  if (S.selected < 0 && delta < 0) return;
  if (S.prefs.view === 'expanded') {
    select(idx);
    const e = S.entries[idx];
    if (!e.is_read) setRead(e, true);
    return;
  }
  if (open) toggleOpen(S.entries[idx].id, true); else select(idx);
}

let scrollTick = false;
function onMainScroll() {
  if (scrollTick || S.prefs.view !== 'expanded') return;
  scrollTick = true;
  requestAnimationFrame(() => {
    scrollTick = false;
    const main = $('#main');
    const top = main.getBoundingClientRect().top + ($('#stream-head')?.offsetHeight || 0);
    let current = -1;
    for (let i = 0; i < S.entries.length; i++) {
      const el = document.querySelector(`.entry[data-id="${S.entries[i].id}"]`);
      if (!el) continue;
      const r = el.getBoundingClientRect();
      if (r.bottom < top + 24) {
        if (S.prefs.markOnScroll && !S.entries[i].is_read) setRead(S.entries[i], true);
      } else if (current < 0) {
        current = i;
        if (r.top > main.getBoundingClientRect().bottom) break;
      } else {
        break;
      }
    }
    if (current >= 0 && current !== S.selected) {
      document.querySelector('.entry.selected')?.classList.remove('selected');
      S.selected = current;
      document.querySelector(`.entry[data-id="${S.entries[current].id}"]`)?.classList.add('selected');
    }
  });
}

/* ---------------- sidebar navigation ---------------- */

function sidebarOrder() {
  const order = [{ kind: 'all' }, { kind: 'starred' }];
  for (const folder of S.tree.folders) {
    order.push({ kind: 'folder', id: folder.id });
    if (!S.prefs.collapsed[folder.id]) {
      for (const f of S.tree.feeds.filter((x) => x.folder_id === folder.id)) order.push({ kind: 'feed', id: f.id });
    }
  }
  for (const f of S.tree.feeds.filter((x) => x.folder_id == null)) order.push({ kind: 'feed', id: f.id });
  return order;
}

function moveSidebar(delta, unreadOnly = false) {
  const order = sidebarOrder();
  const cur = order.findIndex((r) => r.kind === S.route.kind && r.id === S.route.id);
  for (let i = cur + delta; i >= 0 && i < order.length; i += delta) {
    const r = order[i];
    if (unreadOnly && r.kind === 'feed' && !(feedById(r.id)?.unread > 0)) continue;
    if (unreadOnly && r.kind !== 'feed') continue;
    go(r);
    return;
  }
}

/* ---------------- keyboard ---------------- */

const SHORTCUTS = [
  ['j / k', 'Next / previous item (opens it)'],
  ['n / p', 'Move selection without opening'],
  ['o / Enter', 'Open or close selected item'],
  ['s', 'Star / unstar'],
  ['m', 'Mark read / unread'],
  ['v', 'Open original in a new tab'],
  ['l', 'Like on Bluesky / atproto'],
  ['Shift + A', 'Mark all in view as read'],
  ['Shift + J / K', 'Next / previous subscription'],
  ['Shift + N', 'Next subscription with unread items'],
  ['g a / g s', 'Go to All items / Starred'],
  ['1 / 2', 'List view / Expanded view'],
  ['u', 'Toggle unread only'],
  ['r', 'Reload this view'],
  ['Shift + R', 'Refresh all feeds'],
  ['a', 'Add a subscription'],
  ['/', 'Search'],
  ['?', 'Show this help'],
];

function showHelp() {
  dialog({
    title: 'Keyboard shortcuts',
    body: h('div', { class: 'dlg-body' }, h('div', { class: 'kbd-grid' },
      SHORTCUTS.map(([k, d]) => [h('span', {}, k.split(' / ').map((x, i) => [i ? ' / ' : '', h('kbd', {}, x)])), h('span', {}, d)]))),
  });
}

let pendingG = 0;
function onKey(ev) {
  if (!S.user || ev.ctrlKey || ev.metaKey || ev.altKey) return;
  if (ev.target instanceof Element && ev.target.closest('input, textarea, select, [contenteditable]')) return;
  if (document.querySelector('dialog[open]')) return;
  const k = ev.key;
  const cur = S.entries[S.selected];
  if (pendingG && Date.now() - pendingG < 1200) {
    pendingG = 0;
    if (k === 'a') { go({ kind: 'all' }); ev.preventDefault(); return; }
    if (k === 's') { go({ kind: 'starred' }); ev.preventDefault(); return; }
  }
  const actions = {
    j: () => move(1, true),
    k: () => move(-1, true),
    n: () => move(1, false),
    p: () => move(-1, false),
    o: () => cur && (S.prefs.view === 'list' ? toggleOpen(cur.id) : null),
    Enter: () => cur && (S.prefs.view === 'list' ? toggleOpen(cur.id) : null),
    s: () => cur && toggleStar(cur),
    m: () => cur && setRead(cur, !cur.is_read),
    v: () => cur?.url && window.open(cur.url, '_blank', 'noopener,noreferrer'),
    l: () => cur && toggleLike(cur),
    A: () => markViewRead(),
    J: () => moveSidebar(1),
    K: () => moveSidebar(-1),
    N: () => moveSidebar(1, true),
    g: () => { pendingG = Date.now(); },
    1: () => S.prefs.view !== 'list' && setPref('view', 'list'),
    2: () => S.prefs.view !== 'expanded' && setPref('view', 'expanded'),
    u: () => setPref('unreadOnly', !S.prefs.unreadOnly),
    r: () => { loadTree().catch(fail); loadStream(); },
    R: () => refreshAll(),
    a: () => openSubscribe(),
    '/': () => $('#search')?.focus(),
    '?': () => showHelp(),
  };
  const fn = actions[k];
  if (fn) { ev.preventDefault(); fn(); }
}

/* ---------------- settings ---------------- */

function openSettings(tab = 'subscriptions') {
  const tabs = [
    ['subscriptions', 'Subscriptions'],
    ['import', 'Import / export'],
    ['appearance', 'Reading'],
    ['apps', 'Mobile apps'],
    ['bluesky', 'Bluesky'],
    ['account', 'Account'],
  ];
  const body = h('div', { class: 'tab-body' });
  const tabBar = h('div', { class: 'tabs', role: 'tablist' });
  const show = (name) => {
    tabBar.replaceChildren(...tabs.map(([id, label]) => h('button', {
      class: `tab${id === name ? ' active' : ''}`, role: 'tab', 'aria-selected': String(id === name), onclick: () => show(id),
    }, label)));
    body.replaceChildren(...SETTINGS[name]());
  };
  dialog({ title: 'Settings', wide: true, body: h('div', {}, tabBar, body) });
  show(tab);
}

const SETTINGS = {
  subscriptions() {
    const table = h('table', { class: 'subs-table' });
    const rerender = async () => { await loadTree(); fill(); };
    const fill = () => {
      table.replaceChildren(...S.tree.feeds.map((f) => {
        const title = h('input', {
          class: 'input', value: f.title, 'aria-label': 'Title',
          onchange: async () => { try { await api('PATCH', `/feeds/${f.id}`, { title: title.value }); await loadTree(); } catch (e) { fail(e); } },
        });
        const sel = folderSelect(f.folder_id);
        sel.addEventListener('change', async () => {
          try {
            const folder_id = await resolveFolder(sel);
            if (folder_id === undefined) { sel.value = f.folder_id ?? ''; return; }
            await api('PATCH', `/feeds/${f.id}`, { folder_id });
            await rerender();
          } catch (e) { fail(e); }
        });
        const bad = f.error_count > 0 && f.last_error;
        return h('tr', {},
          h('td', {}, feedIcon(f)),
          h('td', { style: 'min-width:160px' }, title),
          h('td', { class: 'hide-sm', style: 'min-width:130px' }, sel),
          h('td', { class: 'url' }, f.url),
          h('td', { class: `status ${bad ? 'bad' : 'ok'}`, title: bad ? f.last_error : `Last checked ${agoLong(f.last_fetched_at)}` },
            bad ? `⚠ ${f.error_count} error${f.error_count === 1 ? '' : 's'}` : agoLong(f.last_fetched_at)),
          h('td', { class: 'actions' },
            h('button', { class: 'btn ghost icon', title: 'Refresh now', 'aria-label': 'Refresh now', onclick: async () => { await refreshFeed(f); fill(); } }, icon('refresh', 'sm')),
            h('button', { class: 'btn ghost icon danger', title: 'Unsubscribe', 'aria-label': 'Unsubscribe', onclick: async () => { await unsubscribe(f); fill(); } }, icon('trash', 'sm'))));
      }));
    };
    fill();
    const folders = h('div', { class: 'folder-list' });
    const fillFolders = () => folders.replaceChildren(...S.tree.folders.map((fo) => h('div', { class: 'item' },
      icon('folder', 'sm'), h('span', { style: 'flex:1' }, fo.name),
      h('button', { class: 'btn ghost icon', 'aria-label': 'Rename folder', onclick: async () => { await renameFolder(fo); fillFolders(); fill(); } }, icon('edit', 'sm')),
      h('button', { class: 'btn ghost icon danger', 'aria-label': 'Delete folder', onclick: async () => { await deleteFolder(fo); fillFolders(); fill(); } }, icon('trash', 'sm')))));
    fillFolders();
    return [
      h('section', {},
        h('div', { style: 'display:flex;justify-content:space-between;align-items:center;margin-bottom:8px' },
          h('h4', { style: 'margin:0' }, `${S.tree.feeds.length} subscriptions`),
          h('button', { class: 'btn', onclick: () => { document.querySelector('dialog[open]')?.close(); openSubscribe(); } }, icon('plus', 'sm'), 'Add')),
        S.tree.feeds.length ? table : h('p', {}, 'No subscriptions yet.')),
      h('section', {},
        h('div', { style: 'display:flex;justify-content:space-between;align-items:center;margin-bottom:8px' },
          h('h4', { style: 'margin:0' }, 'Folders'),
          h('button', {
            class: 'btn', onclick: async () => {
              const name = await ask({ title: 'New folder', confirm: 'Create', input: { placeholder: 'Folder name' } });
              if (!name?.trim()) return;
              try { await api('POST', '/folders', { name }); await loadTree(); fillFolders(); fill(); } catch (e) { fail(e); }
            },
          }, icon('plus', 'sm'), 'New folder')),
        S.tree.folders.length ? folders : h('p', {}, 'No folders.')),
    ];
  },

  import() {
    const file = h('input', { type: 'file', accept: '.opml,.xml,text/xml,text/x-opml,application/xml', class: 'hidden' });
    const status = h('p', { class: 'hint' });
    file.addEventListener('change', async () => {
      const f = file.files?.[0];
      if (!f) return;
      status.textContent = `Importing ${f.name}…`;
      try {
        const r = await api('POST', '/opml', await f.text());
        status.textContent = `Imported ${r.added} new subscription${r.added === 1 ? '' : 's'}${r.skipped ? ` (${r.skipped} already present or invalid)` : ''}. They’ll fill in over the next minute.`;
        await loadTree();
        setTimeout(() => loadTree().catch(() => {}), 8000);
        setTimeout(() => { loadTree().catch(() => {}); loadStream(); }, 25000);
      } catch (e) { status.textContent = ''; fail(e); }
      file.value = '';
    });
    return [
      h('section', {},
        h('h4', {}, 'Import'),
        h('p', {}, 'Upload an OPML file exported from FreshRSS (Subscription management → Import / export), Feedly, Inoreader, NetNewsWire, or any other reader. Folders are preserved.'),
        h('button', { class: 'btn primary', onclick: () => file.click() }, icon('upload', 'sm'), 'Choose OPML file…'),
        file, status),
      h('section', {},
        h('h4', {}, 'Export'),
        h('p', {}, 'Download all your subscriptions and folders as OPML.'),
        h('a', { class: 'btn', href: '/api/opml', download: 'yesrss-subscriptions.opml' }, icon('download', 'sm'), 'Download OPML')),
    ];
  },

  appearance() {
    const radio = (key, options, after) => h('div', { class: 'seg', role: 'group' },
      options.map(([value, label]) => h('button', {
        class: `btn${S.prefs[key] === value ? ' on' : ''}`,
        onclick: (ev) => {
          S.prefs[key] = value;
          savePrefs();
          ev.currentTarget.parentElement.querySelectorAll('.btn').forEach((b) => b.classList.remove('on'));
          ev.currentTarget.classList.add('on');
          after?.();
        },
      }, label)));
    const check = (key, label, after) => h('label', { class: 'check-row' },
      h('input', { type: 'checkbox', checked: !!S.prefs[key], onchange: (ev) => { S.prefs[key] = ev.target.checked; savePrefs(); after?.(); } }),
      label);
    return [
      h('section', {}, h('h4', {}, 'Theme'), radio('theme', [['auto', 'System'], ['light', 'Light'], ['dark', 'Dark']])),
      h('section', {}, h('h4', {}, 'Density'), radio('density', [['compact', 'Compact'], ['normal', 'Normal'], ['comfortable', 'Comfortable']])),
      h('section', {}, h('h4', {}, 'Default view'), radio('view', [['list', 'List'], ['expanded', 'Expanded']], loadStream)),
      h('section', {}, h('h4', {}, 'Behavior'),
        check('markOnScroll', 'In expanded view, mark items as read when scrolled past'),
        check('unreadOnly', 'Show only unread items', loadStream),
        check('oldest', 'Show oldest items first', loadStream)),
      h('section', {}, h('h4', {}, 'Keyboard'), h('p', {}, 'Press ? anywhere to see shortcuts.'),
        h('button', { class: 'btn', onclick: showHelp }, icon('keyboard', 'sm'), 'Show shortcuts')),
    ];
  },

  apps() {
    const url = `${location.origin}/api/greader`;
    const copy = (text) => navigator.clipboard?.writeText(text).then(() => toast('Copied'), () => toast('Copy failed', 'error'));
    const field = h('input', { class: 'input', readonly: true, value: url, onfocus: (ev) => ev.target.select() });
    return [
      h('section', {},
        h('h4', {}, 'Google Reader API'),
        h('p', {}, 'yesrss speaks the same Google Reader API as FreshRSS, so mobile and desktop apps sync read and starred state with it. In your app, choose “FreshRSS” or “Google Reader API” and enter:'),
        h('div', { class: 'copy-row' }, field, h('button', { class: 'btn', onclick: () => copy(url) }, 'Copy')),
        h('p', { class: 'hint', style: 'margin-top:8px' }, `Sign in with your yesrss username (${S.user.username}) and password. Apps already pointed at FreshRSS’s /api/greader.php only need the hostname changed.`)),
      h('section', {},
        h('h4', {}, 'Known-good apps'),
        h('p', {}, 'Reeder, NetNewsWire, ReadYou, FeedMe, FocusReader, Fluent Reader, Newsflash, Vienna, lire.')),
    ];
  },

  bluesky() {
    if (!S.atproto.enabled) {
      return [h('section', {},
        h('h4', {}, 'Connect Bluesky / atproto'),
        h('p', {}, 'Liking items publishes them to your atproto account, so they show up in Skyreader and other apps that read the same records. yesrss can also find standard.site publications by people you follow.'),
        h('p', {}, 'Bluesky sign-in needs this server on a public HTTPS address (a reverse proxy or tunnel such as Pangolin, Caddy or Tailscale Funnel). Start yesrss with:'),
        h('div', { class: 'copy-row' }, h('input', { class: 'input', readonly: true, value: 'YESRSS_PUBLIC_URL=https://rss.example.com' })))];
    }
    const acct = S.atproto.account;
    if (!acct) {
      const handle = h('input', { class: 'input', placeholder: 'you.bsky.social', autocapitalize: 'none', spellcheck: 'false', required: true });
      const err = h('div', { class: 'error-text' });
      const btn = h('button', { class: 'btn primary', type: 'submit' }, icon('butterfly', 'sm'), 'Connect');
      return [h('section', {},
        h('h4', {}, 'Connect your atproto account'),
        h('p', {}, 'You’ll sign in on your own server (bsky.social or your PDS) and come right back. yesrss never sees your password.'),
        h('form', {
          onsubmit: async (ev) => {
            ev.preventDefault();
            err.textContent = '';
            btn.disabled = true;
            try {
              const r = await api('POST', '/atproto/connect', { handle: handle.value });
              location.href = r.redirect;
            } catch (e2) { err.textContent = e2.message; btn.disabled = false; }
          },
        }, h('div', { class: 'field' }, h('label', {}, 'Handle'), handle), err, btn),
        h('p', { class: 'hint', style: 'margin-top:12px' }, 'Likes are public records in your account (app.skyreader.social.recommend, plus site.standard.graph.recommend for standard.site articles).'))];
    }
    const list = h('div', { class: 'pub-list' });
    const loadBtn = h('button', {
      class: 'btn', onclick: async () => {
        loadBtn.disabled = true;
        list.replaceChildren(h('p', { class: 'hint' }, 'Checking who you follow… this can take a few seconds.'));
        try {
          const { publications } = await api('GET', '/atproto/publications');
          list.replaceChildren(...(publications.length ? publications.map((p) => {
            const sub = h('button', {
              class: `btn${p.subscribed ? '' : ' primary'}`, disabled: p.subscribed,
              onclick: async () => {
                sub.disabled = true;
                sub.textContent = 'Subscribing…';
                try {
                  await api('POST', '/feeds', { url: p.url });
                  sub.textContent = 'Subscribed';
                  sub.classList.remove('primary');
                  await loadTree();
                } catch (e2) { sub.disabled = false; sub.textContent = 'Subscribe'; fail(e2); }
              },
            }, p.subscribed ? 'Subscribed' : 'Subscribe');
            return h('div', { class: 'pub' },
              h('div', { class: 'pub-text' },
                h('b', {}, p.name),
                h('small', {}, `@${p.handle} · `, h('a', { href: p.url, target: '_blank', rel: 'noopener noreferrer' }, p.url.replace(/^https?:\/\//, '').replace(/\/$/, ''))),
                p.description && h('span', { class: 'hint' }, p.description)),
              sub);
          }) : [h('p', {}, 'Nobody you follow has a standard.site publication yet.')]));
        } catch (e2) { list.replaceChildren(); fail(e2); } finally { loadBtn.disabled = false; }
      },
    }, icon('search', 'sm'), 'Find publications');
    return [
      h('section', {},
        h('h4', {}, 'Connected'),
        h('p', {}, 'Signed in as ', h('b', {}, `@${acct.handle}`), h('span', { class: 'hint' }, ` (${acct.did})`)),
        h('div', { class: 'radio-row' },
          h('button', {
            class: 'btn danger', onclick: async () => {
              const ok = await ask({ title: 'Disconnect Bluesky?', message: 'Existing likes stay in your account. yesrss will forget its session.', confirm: 'Disconnect', danger: true });
              if (!ok) return;
              try { await api('DELETE', '/atproto'); await loadAtproto(); document.querySelector('dialog[open]')?.close(); openSettings('bluesky'); loadStream(); } catch (e2) { fail(e2); }
            },
          }, icon('logout', 'sm'), 'Disconnect'))),
      h('section', {},
        h('h4', {}, 'From people you follow'),
        h('p', {}, 'standard.site publications by accounts you follow on Bluesky.'),
        loadBtn, list),
    ];
  },

  account() {
    const cur = h('input', { class: 'input', type: 'password', autocomplete: 'current-password', required: true });
    const next = h('input', { class: 'input', type: 'password', autocomplete: 'new-password', minlength: '8', required: true });
    const err = h('div', { class: 'error-text' });
    return [
      h('section', {},
        h('h4', {}, `Signed in as ${S.user.username}`),
        h('button', {
          class: 'btn', onclick: async () => {
            try { await api('POST', '/logout'); } catch { /* already signed out */ }
            S.user = null;
            document.querySelector('dialog[open]')?.close();
            renderLogin();
          },
        }, icon('logout', 'sm'), 'Sign out')),
      h('section', {},
        h('h4', {}, 'Change password'),
        h('p', {}, 'Changing your password signs out every other browser and app.'),
        h('form', {
          onsubmit: async (ev) => {
            ev.preventDefault();
            err.textContent = '';
            try {
              await api('POST', '/password', { current: cur.value, new: next.value });
              cur.value = ''; next.value = '';
              toast('Password changed');
            } catch (e) { err.textContent = e.message; }
          },
        },
        h('div', { class: 'field' }, h('label', {}, 'Current password'), cur),
        h('div', { class: 'field' }, h('label', {}, 'New password'), next),
        err,
        h('button', { class: 'btn primary', type: 'submit' }, 'Change password'))),
    ];
  },
};

/* ---------------- boot ---------------- */

async function loadAtproto() {
  try { S.atproto = await api('GET', '/atproto'); } catch { S.atproto = { enabled: false, account: null }; }
}

let treeTimer = 0;
async function boot() {
  const connected = location.hash === '#/connected';
  if (connected) history.replaceState(null, '', '#/all');
  renderShell();
  await Promise.all([loadTree().catch(fail), loadAtproto()]);
  await loadStream();
  if (connected) {
    openSettings('bluesky');
    if (S.atproto.account) toast(`Connected as @${S.atproto.account.handle}`);
  }
  clearInterval(treeTimer);
  treeTimer = setInterval(() => { if (S.user && !document.hidden) loadTree().catch(() => {}); }, 90_000);
}

async function start() {
  applyPrefs();
  window.addEventListener('hashchange', () => { if (S.user) loadStream(); });
  document.addEventListener('keydown', onKey);
  document.addEventListener('visibilitychange', () => { if (!document.hidden && S.user) loadTree().catch(() => {}); });
  try {
    const r = await api('GET', '/session');
    S.user = r.user;
    boot();
  } catch {
    try {
      const { needed } = await api('GET', '/setup');
      if (needed) renderSetup(); else renderLogin();
    } catch (e) {
      $('#app').replaceChildren(h('div', { class: 'auth' }, h('div', { class: 'auth-card' }, h('h1', {}, 'Cannot reach server'), h('p', {}, e.message))));
    }
  }
}

start();
