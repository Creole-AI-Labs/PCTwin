"""Find hardcoded (untranslated) visible text in every PCTwin screen state.

Usage: python find_hardcoded.py <app-dir> --lang fr|ar [--port 4318] [--out FILE.json] [--only ID,ID]

<app-dir> must be a COPY of the app (never the repository) whose src/locales/<lang>/main.ftl was
replaced by a pseudo-locale made with l10n/pseudo.mjs (fr = accented and bracketed, ar = Arabic
letters and bracketed). Serves it with vite on --port (never 1420), opens
screens.html?sample=<id>&lang=<lang> for every state in the page's picker, and lists every visible
text node and text attribute (aria-label, title, alt, placeholder) that still contains English words.
Whatever comes from a translated string is fully pseudo, so any ASCII word left is either hardcoded
English in the screen code or sample data (names, file names) passed in as a value; the report
gives both so a person can tell them apart. Exit code 1 if anything is found.
Needs python playwright with Chromium.
"""
import argparse, json, os, re, subprocess, sys, time, urllib.request

JS = r"""
() => {
  const frame = document.querySelector('.screens-frame');
  if (!frame) return {error: 'no .screens-frame'};
  const out = [];
  const visible = (el) => {
    for (let p = el; p && p !== document.body; p = p.parentElement) {
      const cs = getComputedStyle(p);
      if (cs.display === 'none' || cs.visibility === 'hidden') return false;
    }
    return true;
  };
  const desc = (el) => {
    let s = el.tagName.toLowerCase();
    if (el.className && typeof el.className === 'string') s += '.' + el.className.trim().split(/\s+/).slice(0, 2).join('.');
    return s;
  };
  const walker = document.createTreeWalker(frame, NodeFilter.SHOW_TEXT);
  let n;
  while ((n = walker.nextNode())) {
    const t = (n.nodeValue || '').replace(/[\u200e\u200f]/g, '').replace(/\s+/g, ' ').trim();
    if (!t) continue;
    const el = n.parentElement;
    if (!el || /^(SCRIPT|STYLE)$/.test(el.tagName) || !visible(el)) continue;
    out.push({where: 'text', el: desc(el), text: t, inBdi: !!el.closest('bdi')});
  }
  for (const el of frame.querySelectorAll('[aria-label],[title],[alt],[placeholder]')) {
    if (!visible(el)) continue;
    for (const a of ['aria-label', 'title', 'alt', 'placeholder']) {
      const v = el.getAttribute(a);
      if (v && v.trim()) out.push({where: a, el: desc(el), text: v.replace(/[\u200e\u200f]/g, '').trim(), inBdi: false});
    }
  }
  return {items: out};
}
"""

WORD = re.compile(r"[A-Za-z]{2,}")
ISOLATE = re.compile("\u2068[^\u2068\u2069]*\u2069")
# Words that a number or date format may legitimately produce; they are not hardcoded text.
FORMAT_WORDS = {"B", "kB", "MB", "GB", "TB", "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec", "AM", "PM"}


def own_text(text):
    """The text with every placeholder value (wrapped by Fluent in isolate marks) cut out."""
    prev = None
    while prev != text:
        prev, text = text, ISOLATE.sub(" ", text)
    return text.replace("\u2068", "").replace("\u2069", "")


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('app')
    ap.add_argument('--lang', required=True)
    ap.add_argument('--port', type=int, default=4318)
    ap.add_argument('--out', default=None)
    ap.add_argument('--only', default='')
    a = ap.parse_args()
    if a.port == 1420:
        sys.exit('port 1420 is reserved')
    app = os.path.abspath(a.app)
    vite = os.path.join(app, 'node_modules', 'vite', 'bin', 'vite.js')
    logf = open(os.path.join(os.path.dirname(app), f'vite-hardcoded-{a.lang}.log'), 'w')
    proc = subprocess.Popen(['node', vite, '--port', str(a.port), '--strictPort', '--host', '127.0.0.1'], cwd=app, stdout=logf, stderr=subprocess.STDOUT)
    base = f'http://127.0.0.1:{a.port}'
    found = {}
    try:
        for _ in range(60):
            try:
                urllib.request.urlopen(base + '/screens.html', timeout=2)
                break
            except Exception:
                time.sleep(1)
        else:
            sys.exit('vite did not start')
        from playwright.sync_api import sync_playwright
        with sync_playwright() as pw:
            br = pw.chromium.launch()
            ctx = br.new_context(viewport={'width': 800, 'height': 700}, reduced_motion='reduce')
            pg = ctx.new_page()
            errors = []
            pg.on('pageerror', lambda e: errors.append(str(e)))
            pg.goto(f'{base}/screens.html?lang={a.lang}')
            pg.wait_for_selector('.screens-controls select')
            ids = pg.eval_on_selector_all('.screens-controls select >> nth=0 >> option', 'os => os.map(o => o.value)')
            only = [x for x in a.only.split(',') if x]
            if only:
                ids = [i for i in ids if i in only]
            print('states', len(ids), flush=True)
            for sid in ids:
                pg.goto(f'{base}/screens.html?sample={sid}&lang={a.lang}')
                pg.wait_for_selector('.screens-frame .app-main')
                pg.wait_for_timeout(100)
                r = pg.evaluate(JS)
                for it in r.get('items', []):
                    if it['inBdi']:
                        continue  # a value shown as data (names, paths), not screen text
                    own = own_text(it['text'])
                    words = [w for w in WORD.findall(own) if w not in FORMAT_WORDS]
                    if words:
                        key = (it['where'], own)
                        e = found.setdefault(key, {'where': it['where'], 'text': own.strip(), 'words': words, 'el': it['el'], 'states': []})
                        e['states'].append(sid)
            br.close()
        result = sorted(found.values(), key=lambda e: (-len(e['states']), e['text']))
        if a.out:
            json.dump({'lang': a.lang, 'pageErrors': errors, 'found': result}, open(a.out, 'w', encoding='utf-8'), indent=1, ensure_ascii=False)
        print(f'{len(result)} distinct English-looking texts, {len(errors)} page errors')
        for e in result:
            print(f"{len(e['states']):3d} states  [{e['where']}] {e['el']}  \"{e['text']}\"  e.g. {e['states'][0]}")
        sys.exit(1 if result else 0)
    finally:
        subprocess.run(['taskkill', '/PID', str(proc.pid), '/T', '/F'], capture_output=True)


main()
