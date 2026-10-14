export type Appearance = 'system' | 'light' | 'dark';
export type ColorTheme = 'light' | 'dark';
export type AppearanceState = { preference: Appearance; resolved: ColorTheme };
export const appearanceStorageKey = 'opentag.appearance';

export function parseAppearance(value: unknown): Appearance {
  return value === 'light' || value === 'dark' ? value : 'system';
}
export function resolveAppearance(preference: Appearance, systemDark: boolean): ColorTheme {
  return preference === 'system' ? systemDark ? 'dark' : 'light' : preference;
}

type AppearanceMedia = Pick<MediaQueryList, 'matches' | 'addEventListener' | 'removeEventListener'>;
type AppearanceStorage = Pick<Storage, 'getItem' | 'setItem'>;

/** Theme state has no dependency on workspace rendering or native file state. */
export function createAppearance(options: {
  media: AppearanceMedia;
  storage: AppearanceStorage;
  changed: (state: AppearanceState) => void;
}) {
  let preference: Appearance = 'system';
  try { preference = parseAppearance(options.storage.getItem(appearanceStorageKey)); }
  catch { /* Restricted storage still permits an in-memory appearance choice. */ }
  const state = (): AppearanceState => ({preference, resolved: resolveAppearance(preference, options.media.matches)});
  const systemChanged = () => { if (preference === 'system') options.changed(state()); };
  options.media.addEventListener('change', systemChanged);
  return {
    get state() { return state(); },
    set(value: unknown) {
      preference = parseAppearance(value);
      try { options.storage.setItem(appearanceStorageKey, preference); }
      catch { /* A disabled storage provider must not make the editor unusable. */ }
      options.changed(state());
    },
    dispose() { options.media.removeEventListener('change', systemChanged); },
  };
}
