export interface Countdown {
  start: () => void;
  cancel: () => void;
}

export function createCountdown(
  from: number,
  onTick: (n: number) => void,
  onComplete: () => void,
): Countdown {
  let timerId: ReturnType<typeof setInterval> | null = null;
  let remaining = from - 1;

  return {
    start() {
      remaining = from - 1;
      timerId = setInterval(() => {
        onTick(remaining);
        if (remaining === 0) {
          if (timerId !== null) clearInterval(timerId);
          timerId = null;
          onComplete();
        } else {
          remaining--;
        }
      }, 1000);
    },
    cancel() {
      if (timerId !== null) {
        clearInterval(timerId);
        timerId = null;
      }
    },
  };
}
