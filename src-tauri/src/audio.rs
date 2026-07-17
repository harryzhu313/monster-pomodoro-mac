// Phase 01 Spike 3:rodio 音色验证,算法逐行对齐旧 offscreen/offscreen.js。
// 正式实现(settings 控制、与状态机联动、停止时淡出)在 phase 04。

use std::f32::consts::TAU;
use std::time::Duration;

use rodio::{OutputStream, Sink, Source};

const SR: u32 = 44_100;

fn lcg(seed: u32) -> u32 {
    seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223)
}

/// 棕噪音(对齐旧 offscreen.js:19-33):白噪音过一阶低通衰减高频,×3.5 补偿音量,
/// 整体增益 0.25;800ms 线性淡入,尾部 400ms 线性淡出(对齐旧版 gain ramp)。
struct BrownNoise {
    seed: u32,
    last: f32,
    t: usize,
    dur: f32,
}

impl Iterator for BrownNoise {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let time = self.t as f32 / SR as f32;
        if time >= self.dur {
            return None;
        }
        self.t += 1;

        self.seed = lcg(self.seed);
        let white = (self.seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
        // 简易棕噪音:每个样本对上一样本做低通,高频能量衰减(旧 offscreen.js:31)
        self.last = (self.last + 0.02 * white) / 1.02;

        let mut env = 1.0_f32;
        if time < 0.8 {
            env = time / 0.8;
        }
        let remain = self.dur - time;
        if remain < 0.4 {
            env = env.min(remain / 0.4);
        }
        Some(self.last * 3.5 * 0.25 * env)
    }
}

impl Source for BrownNoise {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.dur))
    }
}

/// 带包络的正弦单音(对齐旧 offscreen.js:81-92 的 tone()):
/// 20ms 线性起音到 peak,随后指数衰减到 0.001,总时长 dur + 0.05s 收尾。
struct EnvSine {
    freq: f32,
    peak: f32,
    dur: f32,
    t: usize,
}

impl Iterator for EnvSine {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        let time = self.t as f32 / SR as f32;
        if time >= self.dur + 0.05 {
            return None;
        }
        self.t += 1;

        const ATTACK: f32 = 0.02;
        let env = if time < ATTACK {
            self.peak * (time / ATTACK)
        } else {
            self.peak * (0.001 / self.peak).powf(((time - ATTACK) / (self.dur - ATTACK)).min(1.0))
        };
        Some((TAU * self.freq * time).sin() * env)
    }
}

impl Source for EnvSine {
    fn current_frame_len(&self) -> Option<usize> {
        None
    }
    fn channels(&self) -> u16 {
        1
    }
    fn sample_rate(&self) -> u32 {
        SR
    }
    fn total_duration(&self) -> Option<Duration> {
        Some(Duration::from_secs_f32(self.dur + 0.05))
    }
}

/// 播 4 秒棕噪音,验证音色与旧版 Web Audio 是否一致
#[tauri::command]
pub fn play_test_noise() {
    std::thread::spawn(|| {
        let Ok((_stream, handle)) = OutputStream::try_default() else {
            eprintln!("audio: 无法打开输出设备");
            return;
        };
        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };
        sink.append(BrownNoise {
            seed: 42,
            last: 0.0,
            t: 0,
            dur: 4.0,
        });
        sink.sleep_until_end();
    });
}

/// 双音 chime(对齐旧 offscreen.js:94-95):E5 → B5,第二音 +120ms 起、略弱,两音重叠
#[tauri::command]
pub fn play_test_chime() {
    std::thread::spawn(|| {
        let Ok((_stream, handle)) = OutputStream::try_default() else {
            eprintln!("audio: 无法打开输出设备");
            return;
        };
        let Ok(sink) = Sink::try_new(&handle) else {
            return;
        };
        let e5 = EnvSine { freq: 659.25, peak: 0.28, dur: 0.55, t: 0 };
        let b5 = EnvSine { freq: 987.77, peak: 0.22, dur: 0.65, t: 0 };
        sink.append(e5.mix(b5.delay(Duration::from_millis(120))));
        sink.sleep_until_end();
    });
}
