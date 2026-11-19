import { onBeforeUnmount, ref, watch, type Ref } from "vue";

export type SpeakSide = "source" | "target";

const TTS_LANG: Record<string, string> = { zh: "zh-CN", en: "en-US" };
const CHUNK_LIMIT = 220;
const CHUNK_BOUNDARIES = ["。", "！", "？", "；", ". ", "! ", "? ", "; ", "\n"];

export function speechSupported(): boolean {
  return typeof window !== "undefined" && "speechSynthesis" in window;
}

export function detectLang(text: string): "zh" | "en" {
  return /[\u3400-\u9fff]/.test(text) ? "zh" : "en";
}

export function ttsLang(code: string): string {
  return TTS_LANG[code] ?? "zh-CN";
}

function pickVoice(lang: string): SpeechSynthesisVoice | null {
  const voices = window.speechSynthesis.getVoices();
  const normalized = lang.toLowerCase();
  const prefix = normalized.slice(0, 2);
  return (
    voices.find((voice) => voice.lang.replace("_", "-").toLowerCase() === normalized) ??
    voices.find((voice) => voice.lang.toLowerCase().startsWith(prefix)) ??
    null
  );
}

// 部分 WebView 对超长 utterance 会静默截断，按句界切成小块依次排队。
function splitForSpeech(text: string): string[] {
  const chunks: string[] = [];
  let rest = text.trim();
  while (rest.length > CHUNK_LIMIT) {
    const head = rest.slice(0, CHUNK_LIMIT);
    let cut = -1;
    for (const mark of CHUNK_BOUNDARIES) {
      cut = Math.max(cut, head.lastIndexOf(mark));
    }
    if (cut < 60) cut = CHUNK_LIMIT - 1;
    chunks.push(rest.slice(0, cut + 1).trim());
    rest = rest.slice(cut + 1).trim();
  }
  if (rest) chunks.push(rest);
  return chunks.filter(Boolean);
}

export function useSpeechPlayback(rate: Ref<number>) {
  const speakingSide = ref<SpeakSide | null>(null);
  let seq = 0;
  let current: { text: string; lang: string; side: SpeakSide } | null = null;

  function cancel(): void {
    seq += 1;
    speakingSide.value = null;
    if (speechSupported()) window.speechSynthesis.cancel();
  }

  function stop(): void {
    current = null;
    cancel();
  }

  function speakChunks(chunks: string[], lang: string, side: SpeakSide): void {
    const run = ++seq;
    window.speechSynthesis.cancel();
    speakingSide.value = side;
    const voice = pickVoice(lang);
    chunks.forEach((chunk, index) => {
      const utterance = new SpeechSynthesisUtterance(chunk);
      utterance.lang = lang;
      if (voice) utterance.voice = voice;
      utterance.rate = rate.value;
      const finish = (): void => {
        if (run === seq && index === chunks.length - 1) speakingSide.value = null;
      };
      utterance.onend = finish;
      utterance.onerror = finish;
      window.speechSynthesis.speak(utterance);
    });
  }

  function start(text: string, lang: string, side: SpeakSide): void {
    const chunks = splitForSpeech(text);
    if (!chunks.length) {
      stop();
      return;
    }
    current = { text, lang, side };
    speakChunks(chunks, lang, side);
  }

  /** 切换指定一侧的朗读；返回 false 表示当前环境不支持语音。 */
  function toggle(text: string, lang: string, side: SpeakSide): boolean {
    if (!speechSupported()) return false;
    if (speakingSide.value === side) {
      stop();
      return true;
    }
    start(text, lang, side);
    return true;
  }

  // 朗读中调整倍率时按新速率重读当前一侧。
  watch(rate, () => {
    if (!current || !speakingSide.value) return;
    const chunks = splitForSpeech(current.text);
    if (chunks.length) speakChunks(chunks, current.lang, current.side);
  });

  onBeforeUnmount(stop);

  return { speakingSide, toggle, stop };
}
