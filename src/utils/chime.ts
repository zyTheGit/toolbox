//! 下载事件提示音：用 WebAudio 合成两声短音，不额外打包音频资源。

let ctx: AudioContext | null = null;

function context(): AudioContext | null {
  if (ctx) return ctx;
  if (typeof window.AudioContext !== "function") return null;
  ctx = new window.AudioContext();
  return ctx;
}

/**
 * 首次点击时提前解锁音频上下文，避免下载结束提示音被自动播放策略静默拦截。
 * 返回清理函数。
 */
export function warmUpChime(): () => void {
  const unlock = (): void => {
    void context()?.resume();
  };
  window.addEventListener("pointerdown", unlock, { once: true });
  return () => window.removeEventListener("pointerdown", unlock);
}

export function playChime(): void {
  const audio = context();
  if (!audio) return;
  if (audio.state === "suspended") void audio.resume();
  const now = audio.currentTime;
  [880, 1174.66].forEach((freq, index) => {
    const oscillator = audio.createOscillator();
    const gain = audio.createGain();
    oscillator.type = "sine";
    oscillator.frequency.value = freq;
    const start = now + index * 0.14;
    gain.gain.setValueAtTime(0, start);
    gain.gain.linearRampToValueAtTime(0.18, start + 0.02);
    gain.gain.exponentialRampToValueAtTime(0.001, start + 0.34);
    oscillator.connect(gain).connect(audio.destination);
    oscillator.start(start);
    oscillator.stop(start + 0.36);
  });
}
