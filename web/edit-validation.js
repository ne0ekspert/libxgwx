// Reuse only the last successful write against the same immutable source bytes.
export function validatedEditCache(build) {
  let previous;
  return (source, patch) => {
    const key = JSON.stringify(patch);
    if (previous?.source === source && previous.key === key) return previous.bytes;
    const bytes = build(source, patch);
    previous = { source, key, bytes };
    return bytes;
  };
}

// Disable Apply immediately, then check the latest draft after typing settles.
// Editors cancel pending checks whenever their source or rendered form changes.
export function deferredValidation(check, report, {delay = 150, isCurrent = () => true} = {}) {
  let timer;
  const cancel = () => {
    clearTimeout(timer);
    timer = undefined;
  };
  const run = () => {
    cancel();
    let error = '';
    try { check(); } catch (failure) { error = String(failure); }
    report(error, false);
  };
  return {
    cancel,
    run,
    schedule() {
      cancel();
      report('', true);
      timer = setTimeout(() => { if (isCurrent()) run(); else cancel(); }, delay);
    },
  };
}
