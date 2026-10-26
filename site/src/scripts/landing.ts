// Copyright 2026 Ferrule contributors
// SPDX-License-Identifier: Apache-2.0
// Landing page behavior: reveals, counters, the diff scan, the terminal and the pipeline stepper.
// Everything renders without this script; it only animates.
import { escapeHtml } from '../lib/highlight';

const reduced = matchMedia('(prefers-reduced-motion: reduce)').matches;

function all<T extends Element = HTMLElement>(selector: string, root: ParentNode = document): T[] {
  return [...root.querySelectorAll<T>(selector)];
}

function byId(id: string): HTMLElement | null {
  return document.getElementById(id);
}

function once(el: Element | null, run: (el: Element) => void, threshold: number): void {
  if (el === null) return;
  const io = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        if (!entry.isIntersecting) continue;
        run(entry.target);
        io.unobserve(entry.target);
      }
    },
    { threshold },
  );
  io.observe(el);
}

function setText(id: string, text: string): void {
  const el = byId(id);
  if (el !== null) el.textContent = text;
}

/* nav */
const nav = byId('nav');
const onScroll = (): void => {
  nav?.classList.toggle('scrolled', window.scrollY > 12);
};
addEventListener('scroll', onScroll, { passive: true });
onScroll();

/* copy buttons */
const CHECK_ICON =
  '<svg aria-hidden="true" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.4"><path d="M5 13l4 4L19 7"/></svg>';
for (const button of all<HTMLButtonElement>('[data-copy]')) {
  button.addEventListener('click', async () => {
    try {
      await navigator.clipboard.writeText(button.dataset.copy ?? '');
    } catch {
      // Clipboard blocked (insecure context, permissions): still show the feedback.
    }
    const icon = button.querySelector('.copy');
    if (icon === null) return;
    const previous = icon.innerHTML;
    button.classList.add('copied');
    icon.innerHTML = CHECK_ICON;
    setTimeout(() => {
      button.classList.remove('copied');
      icon.innerHTML = previous;
    }, 1600);
  });
}

/* bento spotlight */
for (const tile of all('.tile')) {
  tile.addEventListener('pointermove', (event) => {
    const rect = tile.getBoundingClientRect();
    tile.style.setProperty('--mx', `${event.clientX - rect.left}px`);
    tile.style.setProperty('--my', `${event.clientY - rect.top}px`);
  });
}

/* counters */
function countUp(el: Element): void {
  const target = Number((el as HTMLElement).dataset.count ?? '0');
  if (reduced || target === 0) {
    el.textContent = target.toLocaleString('en-US');
    return;
  }
  const start = performance.now();
  const duration = 1600;
  const step = (now: number): void => {
    const p = Math.min(1, (now - start) / duration);
    el.textContent = Math.round(target * (1 - (1 - p) ** 4)).toLocaleString('en-US');
    if (p < 1) requestAnimationFrame(step);
  };
  requestAnimationFrame(step);
}

/* diff scan: line 1 differs (the generator's name), every other line is identical */
const LINE_HEIGHT = 21;
function markLine(a: Element, b: Element, first: boolean): void {
  a.classList.remove('cur');
  b.classList.remove('cur');
  a.classList.add(first ? 'del' : 'ok');
  b.classList.add(first ? 'add' : 'ok');
  const ma = a.querySelector('.mk');
  const mb = b.querySelector('.mk');
  if (ma !== null) ma.textContent = first ? '−' : '=';
  if (mb !== null) mb.textContent = first ? '+' : '=';
  if (first) setText('dDiff', '1');
}

function runDiff(): void {
  const left = all('#paneA .ln');
  const right = all('#paneB .ln');
  const scrollers = all('#paneA, #paneB');
  const body = byId('paneA')?.parentElement;
  const total = Math.min(left.length, right.length);
  const scrollTo = (offset: number): void => {
    for (const s of scrollers) s.style.transform = `translateY(${-offset}px)`;
  };
  if (reduced || body == null) {
    left.forEach((a, k) => {
      const b = right[k];
      if (b !== undefined) markLine(a, b, k === 0);
    });
    setText('dCount', String(total));
    setText('dApi', '0 breaking');
    return;
  }
  let i = 0;
  const tick = (): void => {
    const prevA = left[i - 1];
    const prevB = right[i - 1];
    if (prevA !== undefined && prevB !== undefined) markLine(prevA, prevB, i === 1);
    const a = left[i];
    const b = right[i];
    if (i >= total || a === undefined || b === undefined) {
      setText('dApi', '0 breaking');
      setTimeout(() => scrollTo(0), 900);
      return;
    }
    a.classList.add('cur');
    b.classList.add('cur');
    setText('dCount', String(i + 1));
    const visible = body.clientHeight;
    scrollTo(
      Math.max(0, Math.min(i * LINE_HEIGHT - visible * 0.55, total * LINE_HEIGHT - visible + 40)),
    );
    i += 1;
    setTimeout(tick, i < 3 ? 380 : 75);
  };
  tick();
}

/* terminal: a real `ferrule generate` run on the OneBusAway spec and config, twice */
type TermLine = { cmd: string } | { out: string };
const GENERATE = 'uv run ferrule generate --spec openapi.yml --config stainless.yml --out sdk';
const GENERATED =
  'generated <span class="hl">onebusaway</span>: 222 files <span class="o">(26 runtime)</span> sha256 <span class="o">1f9d988f…88d96a</span>';
const TERMINAL: TermLine[] = [
  { cmd: GENERATE },
  { out: GENERATED },
  { out: '' },
  { cmd: `rm -r sdk && ${GENERATE}` },
  { out: GENERATED },
  {
    out: '<span class="z">same digest</span> <span class="o">· byte-identical on every run</span>',
  },
];
const PROMPT = '<span class="pr">$</span> ';
const CARET = '<span class="caret"></span>';

function runTerminal(): void {
  const body = byId('termBody');
  if (body === null) return;
  if (reduced) {
    const lines = TERMINAL.map((l) => ('cmd' in l ? PROMPT + escapeHtml(l.cmd) : l.out));
    body.innerHTML = `${lines.join('\n')}\n${PROMPT}${CARET}`;
    return;
  }
  let done = '';
  let index = 0;
  const render = (extra = ''): void => {
    body.innerHTML = done + extra + CARET;
  };
  const next = (): void => {
    const line = TERMINAL[index];
    index += 1;
    if (line === undefined) {
      done += PROMPT;
      render();
      return;
    }
    if ('out' in line) {
      done += `${line.out}\n`;
      render();
      setTimeout(next, line.out ? 170 + Math.random() * 180 : 260);
      return;
    }
    let typed = 0;
    const type = (): void => {
      render(PROMPT + escapeHtml(line.cmd.slice(0, typed)));
      if (typed < line.cmd.length) {
        typed += 1;
        setTimeout(type, 34 + Math.random() * 40);
      } else {
        done += `${PROMPT}${escapeHtml(line.cmd)}\n`;
        setTimeout(next, 420);
      }
    };
    type();
  };
  next();
}

/* pipeline stepper */
const stages = all('#stages .st');
let active = 0;
let timer: ReturnType<typeof setInterval> | undefined;
const setStage = (k: number): void => {
  stages.forEach((s, j) => {
    s.classList.toggle('on', j === k);
  });
  active = k;
};
const cycle = (): void => {
  timer = setInterval(() => setStage((active + 1) % stages.length), 2600);
};
stages.forEach((stage, k) => {
  stage.addEventListener('pointerenter', () => {
    clearInterval(timer);
    setStage(k);
  });
  stage.addEventListener('pointerleave', cycle);
});

/* observers */
for (const el of all('.reveal')) once(el, (t) => t.classList.add('in'), 0.12);
for (const el of all('[data-count]')) once(el, countUp, 0.5);
once(byId('diff'), runDiff, 0.35);
once(byId('term'), runTerminal, 0.4);
once(byId('never'), (t) => t.classList.add('in'), 0.4);
once(
  byId('board'),
  () => {
    all('#boardRows .bar i').forEach((bar, k) => {
      setTimeout(() => {
        bar.style.width = `${bar.dataset.w ?? '0'}%`;
      }, k * 60);
    });
  },
  0.2,
);
once(
  byId('languages'),
  () => {
    all('#langRows .bar i').forEach((bar, k) => {
      setTimeout(() => {
        bar.style.width = `${bar.dataset.w ?? '0'}%`;
      }, k * 60);
    });
  },
  0.2,
);
once(
  byId('stages'),
  () => {
    if (!reduced) cycle();
  },
  0.3,
);

/* One request fills every star count. A failed fetch leaves the labels alone. */
const slots = all<HTMLElement>('[data-stars]');
if (slots.length > 0) {
  void fetch('https://api.github.com/repos/shrijayan/ferrule', {
    headers: { Accept: 'application/vnd.github+json' },
  })
    .then(async (response) => {
      if (!response.ok) return;
      const body: unknown = await response.json();
      if (typeof body !== 'object' || body === null || !('stargazers_count' in body)) return;
      const count = body.stargazers_count;
      if (typeof count !== 'number' || !Number.isInteger(count) || count < 0) return;
      const text = count.toLocaleString('en-US');
      for (const slot of slots) {
        const num = slot.querySelector('.gh-count');
        if (num === null) continue;
        num.textContent = text;
        slot.hidden = false;
      }
    })
    .catch(() => {
      // The buttons already say GitHub / Star on GitHub and already link.
    });
}
