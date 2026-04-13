// Theme toggle — persists to localStorage as "sk-theme".
// The pre-paint snippet in shell.html applies the stored value; this file wires the button.
(function () {
  var root = document.documentElement;
  var btn = document.querySelector('[data-sk-theme-toggle]');
  var label = document.querySelector('[data-sk-theme-label]');
  if (!btn) return;

  function current() {
    return root.getAttribute('data-theme') === 'dark' ? 'dark' : 'light';
  }

  function render() {
    var t = current();
    if (label) label.textContent = t === 'dark' ? 'Dark' : 'Light';
    btn.setAttribute('aria-pressed', t === 'dark' ? 'true' : 'false');
  }

  btn.addEventListener('click', function () {
    var next = current() === 'dark' ? 'light' : 'dark';
    root.setAttribute('data-theme', next);
    try { localStorage.setItem('sk-theme', next); } catch (_) {}
    render();
  });

  render();
})();
