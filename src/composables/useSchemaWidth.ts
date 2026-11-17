import { onBeforeUnmount, ref } from "vue";

const STORAGE_KEY = "toolbox.db.schemaWidth";
const DEFAULT_WIDTH = 268;
const MIN_WIDTH = 200;
const MAX_WIDTH = 480;

function readWidth(): number {
  try {
    const raw = Number(localStorage.getItem(STORAGE_KEY));
    if (Number.isFinite(raw) && raw >= MIN_WIDTH && raw <= MAX_WIDTH) {
      return raw;
    }
  } catch {
    // 本地存储不可用时使用默认宽度
  }
  return DEFAULT_WIDTH;
}

export function useSchemaWidth() {
  const schemaWidth = ref(readWidth());
  const dragging = ref(false);
  let drag: { startX: number; startWidth: number } | null = null;

  function startResize(event: PointerEvent): void {
    event.preventDefault();
    drag = { startX: event.clientX, startWidth: schemaWidth.value };
    dragging.value = true;
    document.body.style.cursor = "col-resize";
    document.body.style.userSelect = "none";
    window.addEventListener("pointermove", onResize);
    window.addEventListener("pointerup", stopResize, { once: true });
    window.addEventListener("pointercancel", stopResize, { once: true });
  }

  function onResize(event: PointerEvent): void {
    if (!drag) return;
    const next = drag.startWidth + event.clientX - drag.startX;
    schemaWidth.value = Math.max(MIN_WIDTH, Math.min(MAX_WIDTH, Math.round(next)));
  }

  function stopResize(): void {
    if (drag) {
      try {
        localStorage.setItem(STORAGE_KEY, String(schemaWidth.value));
      } catch {
        // 本地存储不可用时仅本次生效
      }
    }
    drag = null;
    dragging.value = false;
    document.body.style.cursor = "";
    document.body.style.userSelect = "";
    window.removeEventListener("pointermove", onResize);
    window.removeEventListener("pointerup", stopResize);
    window.removeEventListener("pointercancel", stopResize);
  }

  onBeforeUnmount(stopResize);

  return { schemaWidth, dragging, startResize };
}
