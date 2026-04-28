import { describe, it, expect, vi, beforeEach, afterEach } from "vite-plus/test";
import { createCountdown } from "~/lib/countdown";

describe("createCountdown", () => {
  beforeEach(() => vi.useFakeTimers());
  afterEach(() => vi.useRealTimers());

  it("onTick を各秒に呼ぶ", () => {
    const ticks: number[] = [];
    const { start } = createCountdown(3, (n) => ticks.push(n), vi.fn());
    start();
    vi.advanceTimersByTime(3000);
    expect(ticks).toEqual([2, 1, 0]);
  });

  it("onComplete を最終tick後に呼ぶ", () => {
    const onComplete = vi.fn();
    const { start } = createCountdown(3, vi.fn(), onComplete);
    start();
    vi.advanceTimersByTime(3000);
    expect(onComplete).toHaveBeenCalledOnce();
  });

  it("cancel でインターバルが止まる", () => {
    const ticks: number[] = [];
    const { start, cancel } = createCountdown(5, (n) => ticks.push(n), vi.fn());
    start();
    vi.advanceTimersByTime(2000);
    cancel();
    vi.advanceTimersByTime(3000);
    expect(ticks).toHaveLength(2);
  });

  it("onComplete は cancel 後には呼ばれない", () => {
    const onComplete = vi.fn();
    const { start, cancel } = createCountdown(3, vi.fn(), onComplete);
    start();
    vi.advanceTimersByTime(1000);
    cancel();
    vi.advanceTimersByTime(2000);
    expect(onComplete).not.toHaveBeenCalled();
  });
});
