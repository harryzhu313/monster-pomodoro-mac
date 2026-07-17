// 音频引擎:专用线程持有 OutputStream,经 channel 接收指令(ARCHITECTURE §5)。
// 算法逐行对齐旧 offscreen/offscreen.js:
// - 白噪音 = 棕噪音(一阶低通)循环源;起播 800ms 淡入,停止 400ms 淡出后才返回,
//   保证后续 chime 不与噪音尾巴重叠(旧版"等淡出完成再回应"的行为)
// - chime = E5→B5 重叠双音;soft nudge = G5→C6(最后一分钟用)
// 设置开关的判断在 lib.rs 效果执行器,引擎只管播放。

use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::time::{Duration, Instant};

use rodio::{OutputStream, OutputStreamHandle, Sink, Source};

const SR: u32 = 44_100;

#[derive(Debug)]
pub enum AudioCmd {
    /// delay 用于与 chime 错峰(休息开始后 1 秒起播,旧版行为)
    NoiseStart { delay: Duration },
    NoiseStop,
    Chime,
    SoftNudge,
}

pub struct AudioEngine {
    tx: Sender<AudioCmd>,
}

impl AudioEngine {
    pub fn new() -> AudioEngine {
        let (tx, rx) = channel();
        std::thread::spawn(move || engine_loop(rx));
        AudioEngine { tx }
    }

    pub fn send(&self, cmd: AudioCmd) {
        let _ = self.tx.send(cmd);
    }
}

fn engine_loop(rx: Receiver<AudioCmd>) {
    let Ok((_stream, handle)) = OutputStream::try_default() else {
        eprintln!("audio: 无法打开输出设备,音频全部静默");
        while rx.recv().is_ok() {}
        return;
    };
    let mut noise: Option<Sink> = None;
    let mut pending_noise: Option<Instant> = None;

    loop {
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(AudioCmd::NoiseStart { delay }) => {
                // 已在播/已排期则忽略(状态驱动,重复指令无害)
                if noise.is_none() && pending_noise.is_none() {
                    pending_noise = Some(Instant::now() + delay);
                }
            }
            Ok(AudioCmd::NoiseStop) => {
                pending_noise = None;
                if let Some(sink) = noise.take() {
                    fade(&sink, 1.0, 0.0, Duration::from_millis(400));
                    sink.stop();
                }
            }
            Ok(AudioCmd::Chime) => play_chime(&handle),
            Ok(AudioCmd::SoftNudge) => play_nudge(&handle),
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        if let Some(at) = pending_noise {
            if Instant::now() >= at {
                pending_noise = None;
                noise = start_noise(&handle);
            }
        }
    }
}

/// 线性音量渐变(阻塞引擎线程;期间到达的指令排队,语义与旧版"等淡出完成"一致)
fn fade(sink: &Sink, from: f32, to: f32, dur: Duration) {
    const STEPS: u32 = 20;
    for i in 0..=STEPS {
        let t = i as f32 / STEPS as f32;
        sink.set_volume(from + (to - from) * t);
        std::thread::sleep(dur / STEPS);
    }
}

fn start_noise(handle: &OutputStreamHandle) -> Option<Sink> {
    let sink = Sink::try_new(handle).ok()?;
    sink.set_volume(0.0);
    sink.append(BrownNoise { seed: 0x2a, last: 0.0 });
    fade(&sink, 0.0, 1.0, Duration::from_millis(800));
    Some(sink)
}

fn play_chime(handle: &OutputStreamHandle) {
    let Ok(sink) = Sink::try_new(handle) else { return };
    let e5 = EnvSine { freq: 659.25, peak: 0.28, dur: 0.55, t: 0 };
    let b5 = EnvSine { freq: 987.77, peak: 0.22, dur: 0.65, t: 0 };
    sink.append(e5.mix(b5.delay(Duration::from_millis(120))));
    sink.detach();
}

/// 临近结束的轻提示:两声短音,比 chime 短但足够听见(旧 playSoftNudge)
fn play_nudge(handle: &OutputStreamHandle) {
    let Ok(sink) = Sink::try_new(handle) else { return };
    let g5 = EnvSine { freq: 783.99, peak: 0.20, dur: 0.42, t: 0 };
    let c6 = EnvSine { freq: 1046.5, peak: 0.18, dur: 0.48, t: 0 };
    sink.append(g5.mix(c6.delay(Duration::from_millis(240))));
    sink.detach();
}

/// 棕噪音无限循环源(旧 offscreen.js:19-33):白噪音过一阶低通,×3.5 补偿,×0.25 增益;
/// 淡入淡出由 Sink 音量控制,源本身不带包络
struct BrownNoise {
    seed: u32,
    last: f32,
}

impl Iterator for BrownNoise {
    type Item = f32;

    fn next(&mut self) -> Option<f32> {
        self.seed = self.seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        let white = (self.seed >> 8) as f32 / (1 << 24) as f32 * 2.0 - 1.0;
        self.last = (self.last + 0.02 * white) / 1.02;
        Some(self.last * 3.5 * 0.25)
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
        None
    }
}

/// 带包络正弦单音(旧 offscreen.js tone()):20ms 线性起音,指数衰减到 0.001
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
        Some((std::f32::consts::TAU * self.freq * time).sin() * env)
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
