import { describe, expect, it, vi, beforeEach, afterEach } from "vite-plus/test";
import { runCountdown, delay } from "~/hooks/useCountdown";
import { countdownValue } from "~/state/signals";

beforeEach(() => vi.useFakeTimers());
afterEach(() => vi.useRealTimers());

describe("delay", () => {
  it("指定時間後に resolve する", async () => {
    const p = delay(200);
    vi.advanceTimersByTime(200);
    await p;
  });

  it("時間が来るまでは resolve しない", () => {
    let resolved = false;
    delay(500).then(() => {
      resolved = true;
    });
    vi.advanceTimersByTime(499);
    expect(resolved).toBe(false);
  });
});

describe("runCountdown", () => {
  it("from=1 でカウントダウンが完了する", async () => {
    const p = runCountdown(1);
    await vi.advanceTimersByTimeAsync(1000);
    await p;
  });

  it("tickCountdown を通じて countdownValue が更新される", async () => {
    const p = runCountdown(3);
    await vi.advanceTimersByTimeAsync(1000);
    expect(countdownValue.value).toBe(2);
    await vi.advanceTimersByTimeAsync(2000);
    await p;
    expect(countdownValue.value).toBe(0);
  });
});
