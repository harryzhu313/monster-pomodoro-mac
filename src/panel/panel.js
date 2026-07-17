// Phase 01 spike:只有两个音频测试按钮。正式面板逻辑(state-update 订阅)在 phase 03。
const { invoke } = window.__TAURI__.core;

document.getElementById('noise').addEventListener('click', () => invoke('play_test_noise'));
document.getElementById('chime').addEventListener('click', () => invoke('play_test_chime'));
