export function useVerticalSplit({
  containerRef,
  enabled,
  storageKey = "piano-roll-height",
  initialBottomHeight = 320,
  minTopHeight = 180,
  minBottomHeight = 140,
  dividerHeight = 8,
}: {
  containerRef: Ref<HTMLElement | null>;
  enabled: Ref<boolean>;
  storageKey?: string;
  initialBottomHeight?: number;
  minTopHeight?: number;
  minBottomHeight?: number;
  dividerHeight?: number;
}) {
  const bottomHeight = useLocalStorage<number>(storageKey, initialBottomHeight);
  const resizing = ref(false);

  let startClientY = 0;
  let startBottomHeight = 0;
  let prevBodyUserSelect = "";

  function getContainerHeight(): number {
    const element = containerRef.value;
    if (element) return element.clientHeight;
    return window.innerHeight;
  }

  function clampBottomHeight(value: number): number {
    const maxBottomHeight = Math.max(
      getContainerHeight() - minTopHeight - dividerHeight,
      minBottomHeight,
    );
    return Math.min(Math.max(value, minBottomHeight), maxBottomHeight);
  }

  const clampedBottomHeight = computed(() => clampBottomHeight(bottomHeight.value));

  watch(clampedBottomHeight, (value) => {
    if (Math.abs(value - bottomHeight.value) > 0.5) {
      bottomHeight.value = value;
    }
  }, { immediate: true });

  const gridTemplateRows = computed(() => {
    if (!enabled.value) return "minmax(0,1fr)";
    const heightPx = Math.round(clampedBottomHeight.value);
    return `minmax(0,1fr) ${dividerHeight}px ${heightPx}px`;
  });

  function stopResize() {
    if (!resizing.value) return;
    resizing.value = false;
    document.body.style.userSelect = prevBodyUserSelect;
  }

  function onPointerMove(event: PointerEvent) {
    if (!resizing.value) return;

    event.preventDefault();
    const deltaY = event.clientY - startClientY;
    bottomHeight.value = clampBottomHeight(startBottomHeight - deltaY);
  }

  function startResize(event: PointerEvent) {
    if (!enabled.value) return;

    event.preventDefault();
    resizing.value = true;
    startClientY = event.clientY;
    startBottomHeight = clampedBottomHeight.value;

    prevBodyUserSelect = document.body.style.userSelect;
    document.body.style.userSelect = "none";

    const target = event.currentTarget;
    if (target instanceof HTMLElement) {
      target.setPointerCapture(event.pointerId);
    }
  }

  useEventListener(window, "pointermove", onPointerMove, { passive: false });
  useEventListener(window, "pointerup", stopResize);
  useEventListener(window, "blur", stopResize);
  useEventListener(window, "resize", () => {
    bottomHeight.value = clampBottomHeight(bottomHeight.value);
  });

  onBeforeUnmount(() => {
    stopResize();
  });

  return {
    resizing: readonly(resizing),
    gridTemplateRows,
    startResize,
  };
}
