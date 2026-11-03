import { byId, setText } from '../dom';
import { writeHash } from '../settings/hash';
import { planLink, SHARED_NAME, sharedInputOf, type LinkPlan } from '../share/link';
import type { AppState } from '../state';
import type { Store } from '../store';
import type { Loader } from './load-input';

type Codec = typeof import('../share/codec');

const COPIED_MS = 1200;
const LABEL = 'Copy link';
const OPENED = 'Opened from a link. Press Run.';

const NOTES: Readonly<Record<'code' | 'empty' | 'large', string>> = {
  code: 'Link copied with the code and settings. Nothing is uploaded.',
  empty: 'The input is empty; the link holds the settings only.',
  large: 'Too large for a link; it holds the settings only.',
};

function message(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

function urlOf(hash: string): string {
  return `${window.location.origin}${window.location.pathname}${window.location.search}${hash}`;
}

function noteOf(plan: LinkPlan): string {
  return plan.kind === 'code' ? NOTES.code : NOTES[plan.reason];
}

function addProblem(store: Store<AppState>, problem: string): void {
  store.update((state) => ({ ...state, settingsProblems: [...state.settingsProblems, problem] }));
}

function wireCopy(store: Store<AppState>, loader: Loader): void {
  const label = byId('share-link-label');
  let timer = 0;
  const flash = (text: string) => {
    setText(label, text);
    window.clearTimeout(timer);
    timer = window.setTimeout(() => setText(label, LABEL), COPIED_MS);
  };
  let codec: Codec | null = null;
  let loading: Promise<Codec> | null = null;
  const load = (): Promise<Codec> => {
    loading ??= import('../share/codec').then(
      (module) => {
        codec = module;
        return module;
      },
      (error: unknown) => {
        loading = null;
        throw error;
      },
    );
    return loading;
  };
  const copyWith = (module: Codec): Promise<LinkPlan> => {
    const state = store.get();
    const plan = planLink(writeHash(state.settings), state.input, module.packCode);
    return navigator.clipboard.writeText(urlOf(plan.hash)).then(() => plan);
  };
  const button = byId('share-link');
  for (const warm of ['pointerenter', 'pointerdown', 'focus']) {
    button.addEventListener(warm, () => void load().catch(() => undefined));
  }
  button.addEventListener('click', () => {
    (codec === null ? load().then(copyWith) : copyWith(codec)).then(
      (plan) => {
        flash('Copied');
        loader.note(noteOf(plan));
      },
      (error: unknown) => {
        flash('Copy failed');
        loader.note(`The link could not be copied: ${message(error)}`);
      },
    );
  });
}

function openFrom(store: Store<AppState>, loader: Loader, hash: string): void {
  const shared = sharedInputOf(hash);
  if (shared === null) {
    return;
  }
  const settingsHash = writeHash(store.get().settings);
  window.history.replaceState(null, '', settingsHash === '' ? window.location.pathname + window.location.search : settingsHash);
  import('../share/codec')
    .then(({ unpackCode }) => unpackCode(shared.code))
    .then(
      (text) => loader.open(text, shared.name ?? SHARED_NAME, OPENED),
      (error: unknown) => addProblem(store, message(error)),
    );
}

export function mountShareLink(store: Store<AppState>, loader: Loader): void {
  wireCopy(store, loader);
  openFrom(store, loader, window.location.hash);
  window.addEventListener('hashchange', (event) => openFrom(store, loader, new URL(event.newURL).hash));
}
