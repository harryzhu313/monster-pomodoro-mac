// 庆祝弹层交互,平移旧 content/celebration.js:
// 飘心装饰、点击/Esc 关闭、10 秒自动淡出;关闭 = 销毁窗口(临时窗口不复用)。

const tauri = window.__TAURI__ ?? null;

const $overlay = document.getElementById('overlay');
const $hearts = document.getElementById('hearts');

const HEART_GLYPHS = ['♥', '❤', '💗', '💖', '💕'];
for (let i = 0; i < 22; i++) {
  const h = document.createElement('span');
  h.className = 'heart';
  h.textContent = HEART_GLYPHS[i % HEART_GLYPHS.length];
  h.style.left = `${Math.random() * 100}%`;
  h.style.fontSize = `${18 + Math.random() * 36}px`;
  h.style.animationDuration = `${4 + Math.random() * 4}s`;
  h.style.animationDelay = `${Math.random() * 4}s`;
  h.style.opacity = `${0.55 + Math.random() * 0.45}`;
  $hearts.appendChild(h);
}

let closed = false;
function close() {
  if (closed) return;
  closed = true;
  $overlay.classList.add('is-leaving');
  setTimeout(() => {
    if (tauri) tauri.core.invoke('close_celebration');
  }, 460);
}

$overlay.addEventListener('click', close);
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') close();
});
setTimeout(close, 10000);
