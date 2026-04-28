import { createCountdown } from "~/lib/countdown";
import { tickCountdown } from "~/hooks/useAppState";

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function runCountdown(from: number): Promise<void> {
  return new Promise((resolve) => {
    const cd = createCountdown(from, tickCountdown, resolve);
    cd.start();
  });
}

export { delay };
