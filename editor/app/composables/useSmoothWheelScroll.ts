export function useSmoothWheelScroll({
  scrollRef,
  lerp = 0.22,
  stopEpsilon = 0.5,
  shouldHandleWheel,
  preventDefaultWhenIgnored = false,
  wheelDirection = "default",
}: {
  scrollRef: Ref<HTMLElement | null>;
  lerp?: number;
  stopEpsilon?: number;
  shouldHandleWheel?: (event: WheelEvent) => boolean;
  preventDefaultWhenIgnored?: boolean;
  wheelDirection?: "default" | "horizontal";
}) {
  let targetLeft = 0;
  let targetTop = 0;
  let rafId: number | null = null;

  function syncTargetsWithElement(element: HTMLElement) {
    targetLeft = element.scrollLeft;
    targetTop = element.scrollTop;
  }

  function clampTargets(element: HTMLElement) {
    const maxLeft = Math.max(element.scrollWidth - element.clientWidth, 0);
    const maxTop = Math.max(element.scrollHeight - element.clientHeight, 0);
    targetLeft = Math.max(0, Math.min(targetLeft, maxLeft));
    targetTop = Math.max(0, Math.min(targetTop, maxTop));
  }

  function stopAnimation() {
    if (rafId != null) {
      cancelAnimationFrame(rafId);
      rafId = null;
    }
  }

  function stepAnimation() {
    const element = scrollRef.value;
    if (!element) {
      stopAnimation();
      return;
    }

    clampTargets(element);

    const currentLeft = element.scrollLeft;
    const currentTop = element.scrollTop;
    const nextLeft = currentLeft + (targetLeft - currentLeft) * lerp;
    const nextTop = currentTop + (targetTop - currentTop) * lerp;
    element.scrollTo(nextLeft, nextTop);

    const actualLeft = element.scrollLeft;
    const actualTop = element.scrollTop;
    const doneX = Math.abs(targetLeft - actualLeft) <= stopEpsilon;
    const doneY = Math.abs(targetTop - actualTop) <= stopEpsilon;

    if (doneX && doneY) {
      element.scrollTo(targetLeft, targetTop);
      stopAnimation();
      return;
    }

    const noProgressX = Math.abs(actualLeft - currentLeft) <= 0.01;
    const noProgressY = Math.abs(actualTop - currentTop) <= 0.01;
    if (noProgressX && noProgressY) {
      syncTargetsWithElement(element);
      stopAnimation();
      return;
    }

    rafId = requestAnimationFrame(stepAnimation);
  }

  function startAnimation() {
    if (rafId != null) return;
    rafId = requestAnimationFrame(stepAnimation);
  }

  useEventListener(
    scrollRef,
    "wheel",
    (event: WheelEvent) => {
      const element = scrollRef.value;
      if (!element) return;
      if (shouldHandleWheel && !shouldHandleWheel(event)) {
        syncTargetsWithElement(element);
        stopAnimation();
        if (preventDefaultWhenIgnored) {
          event.preventDefault();
        }
        return;
      }

      event.preventDefault();

      const deltaScale = event.deltaMode === WheelEvent.DOM_DELTA_LINE
        ? 16
        : event.deltaMode === WheelEvent.DOM_DELTA_PAGE
          ? element.clientHeight
          : 1;

      if (rafId == null) {
        targetLeft = element.scrollLeft;
        targetTop = element.scrollTop;
      }

      const deltaX = event.deltaX * deltaScale;
      const deltaY = event.deltaY * deltaScale;

      if (wheelDirection === "horizontal") {
        if (event.shiftKey) {
          targetLeft += deltaX;
          targetTop += deltaY;
        } else {
          targetLeft += deltaX + deltaY;
        }
      } else {
        if (event.shiftKey && deltaX === 0) {
          targetLeft += deltaY;
        } else {
          targetLeft += deltaX;
          targetTop += deltaY;
        }
      }

      clampTargets(element);
      startAnimation();
    },
    { passive: false },
  );

  onBeforeUnmount(() => {
    stopAnimation();
  });
}
