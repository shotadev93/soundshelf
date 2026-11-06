// Shared by the real WebKit test and deterministic Node regressions. Test-only.
export function waitForDuration(audio, expectedDuration, timeoutMs = 5000) {
  return new Promise((resolve, reject) => {
    const events = ["loadedmetadata", "durationchange", "loadeddata", "canplay", "timeupdate"];
    const cleanup = () => {
      clearTimeout(timeout);
      events.forEach(name => audio.removeEventListener(name, check));
      audio.removeEventListener("error", failed);
    };
    const check = () => {
      // NaN must fail too: Math.abs(NaN - expected) > tolerance is false.
      if (Number.isFinite(audio.duration) && audio.duration > 0 && Math.abs(audio.duration - expectedDuration) <= 0.3) {
        cleanup();
        resolve();
      }
    };
    const failed = () => {
      cleanup();
      reject(new Error(`media error ${audio.error?.code}`));
    };
    const timeout = setTimeout(() => {
      cleanup();
      reject(new Error(`duration did not settle: expected ${expectedDuration}, got ${audio.duration}`));
    }, timeoutMs);
    events.forEach(name => audio.addEventListener(name, check));
    audio.addEventListener("error", failed);
    if (audio.error) failed();
    else check();
  });
}
