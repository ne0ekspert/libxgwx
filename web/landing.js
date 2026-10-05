const copyButton = document.querySelector('#copy-code');
const copyStatus = document.querySelector('#copy-status');
copyButton.addEventListener('click', async () => {
  try {
    await navigator.clipboard.writeText(document.querySelector('#quick-code').textContent);
    copyButton.textContent = 'Copied';
    copyStatus.textContent = 'Copied Cargo.toml dependency and main.rs example.';
  } catch {
    copyStatus.textContent = 'Copy unavailable. Select and copy the code below.';
  }
});
