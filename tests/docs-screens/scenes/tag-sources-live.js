// Docs screenshot scenes for the Tag sources section that need a live provider.
// They run only in a real-music capture (scripts/docs-screenshots-macos.sh),
// against the owner's albums and with the owner's saved keys, and they never
// press Save: every flow ends by closing the dialog.
//
// Albums (copies under OPENTAG_DOCS_MUSIC):
//   albums/R.E.M. - Automatic For The People   a classic album, found everywhere
//   albums/Miles Davis - Kind Of Blue
//   albums/[Æ001D] …                            a DJ release (Beatport / Traxsource)
//   songs/*                                     single tracks (iTunes, Deezer, LRCLIB, AcoustID)
(window.OPENTAG_DOCS_SCENE_FILES = window.OPENTAG_DOCS_SCENE_FILES || []).push(h => {
  if (!window.OPENTAG_DOCS_MUSIC) return [];   // needs the real-music runner
  const REM = 'albums/R.E.M. - Automatic For The People';
  const MILES = 'albums/Miles Davis - Kind Of Blue';
  const q = (d, s) => d.querySelector(s);

  // Search form -> results -> review (fields, then tracks and files). Takes the images named
  // in `shots` ({form, results, review, tracks}) and stops before saving.
  //   mode:      value of the search-type select (#ts-mode)
  //   terms:     { inputId: value } to type before searching (the form is prefilled from the tags)
  //   direct:    the source goes straight to review (per-file lookups such as LRCLIB)
  const searchFlow = async (op, shots, { terms, mode, direct = false, pickRow = 0 } = {}) => {
    const d = await h.openDialog(op, '#ts-search');
    if (mode && q(d, '#ts-mode')) { h.input(q(d, '#ts-mode'), mode); await h.sleep(500); }
    for (const [id, value] of Object.entries(terms || {})) { const el = q(d, '#' + id); if (el) h.input(el, value); }
    await h.sleep(700);
    if (shots.form) await h.snap(shots.form, 'dialog');
    q(d, '#ts-search').click();
    // What the dialog says when a provider fails (shown in the log instead of a bare timeout).
    const said = () => [...d.querySelectorAll('.dialog-error, .ts-status, .ts-hint, [role="alert"], [role="status"]')].map(e => e.textContent.trim()).filter(Boolean).join(' / ').slice(0, 300);
    const failWith = async what => { await h.report(`LOG ${op}: ${what}; dialog says: ${said()}`); throw new Error(`${op}: ${what}; dialog says: ${said()}`); };
    if (!direct) {
      try { await h.waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 60000, op + ' results'); } catch { await failWith('no results'); }
      await h.sleep(1500);                                // thumbnails and long titles settle
      const rows = [...d.querySelectorAll('.ts-results tbody tr')];
      if (rows[pickRow]) rows[pickRow].click();
      await h.sleep(400);
      if (shots.results) await h.snap(shots.results, 'dialog');
      await h.waitFor(() => !q(d, '#ts-next').disabled, 8000, 'Next enabled');
      q(d, '#ts-next').click();
    }
    let apply;
    try { apply = await h.waitFor(() => q(d, '#ts-apply'), 60000, op + ' review'); await h.waitFor(() => !apply.disabled, 60000, 'review loaded'); } catch { await failWith('review did not load'); }
    await h.sleep(1500);
    // The cover thumbnail loads after the fields: wait for it when the release has one.
    await h.waitFor(() => { const img = d.querySelector('.ts-cover-preview img, .ts-review img'); return !img || (img.complete && img.naturalWidth > 0); }, 20000, 'cover loaded').catch(() => {});
    await h.sleep(800);
    if (shots.review) await h.snap(shots.review, 'dialog');
    const tracks = q(d, '.ts-match-section');
    if (tracks && shots.tracks) { tracks.scrollIntoView({ block: 'center' }); await h.sleep(700); await h.snap(shots.tracks, 'dialog'); }
    d.close();
  };

  // A flow starts from freshly read tags and an idle app: a previous provider request can still be
  // winding down, and a new dialog then fails to open.
  const scene = (name, also, album, run) => ({ name, also, album, noFinalShot: true, run: async () => {
    h.op('reload'); await h.sleep(1000); await h.idle(); await h.sleep(1500);
    await run();
  } });

  // Genre-appropriate material: rock albums for MusicBrainz/Discogs/fanart.tv, the Anyma DJ release and
  // EDM songs for the dance stores (Beatport, Traxsource), pop songs for iTunes, Deezer, LRCLIB, AcoustID.
  const DJ = h.music('albums', /Anyma/);
  const EDM = h.music('songs', /Adriatique/);
  const POP = h.music('songs', /Taylor Swift/);
  const LYRICS = h.music('songs', /Lady Gaga/);
  const FINGERPRINT = h.music('songs', /Olivia Dean/);

  return [
    scene('musicbrainz-flow', ['musicbrainz-results', 'musicbrainz-review', 'musicbrainz-tracks'], REM, async () => {
      await h.selectAll();
      await searchFlow('source-musicbrainz', { results: 'musicbrainz-results', review: 'musicbrainz-review', tracks: 'musicbrainz-tracks' });
    }),
    scene('discogs-flow', ['discogs-results', 'discogs-review'], MILES, async () => {
      await h.selectAll();
      await searchFlow('source-discogs', { results: 'discogs-results', review: 'discogs-review' });
    }),
    scene('discogs-release-flow', ['discogs-dj-results', 'discogs-dj-review'], DJ, async () => {
      await h.selectAll();
      await searchFlow('source-discogs', { results: 'discogs-dj-results', review: 'discogs-dj-review' });
    }),
    scene('beatport-release-flow', ['beatport-release-form', 'beatport-release-results', 'beatport-release-review', 'beatport-release-tracks'], DJ, async () => {
      await h.selectAll();
      await searchFlow('source-beatport', { form: 'beatport-release-form', results: 'beatport-release-results', review: 'beatport-release-review', tracks: 'beatport-release-tracks' }, { mode: 'release' });
    }),
    scene('beatport-track-flow', ['beatport-track-results', 'beatport-track-review'], EDM, async () => {
      await h.selectAll();
      await searchFlow('source-beatport', { results: 'beatport-track-results', review: 'beatport-track-review' }, { mode: 'artist_title' });
    }),
    scene('traxsource-release-flow', ['traxsource-release-results', 'traxsource-release-review', 'traxsource-release-tracks'], DJ, async () => {
      await h.selectAll();
      await searchFlow('source-traxsource', { results: 'traxsource-release-results', review: 'traxsource-release-review', tracks: 'traxsource-release-tracks' }, { mode: 'release' });
    }),
    scene('traxsource-track-flow', ['traxsource-track-results', 'traxsource-track-review'], EDM, async () => {
      await h.selectAll();
      await searchFlow('source-traxsource', { results: 'traxsource-track-results', review: 'traxsource-track-review' }, { mode: 'artist_title' });
    }),
    scene('itunes-flow', ['itunes-results', 'itunes-review'], POP, async () => {
      await h.selectAll();
      await searchFlow('source-itunes', { results: 'itunes-results', review: 'itunes-review' });
    }),
    scene('deezer-flow', ['deezer-results', 'deezer-review'], POP, async () => {
      await h.selectAll();
      await searchFlow('source-deezer', { results: 'deezer-results', review: 'deezer-review' });
    }),
    scene('lrclib-flow', ['lrclib-form', 'lrclib-review'], LYRICS, async () => {
      await h.selectAll();
      await searchFlow('source-lrclib', { form: 'lrclib-form', review: 'lrclib-review' }, { direct: true });
    }),
    scene('acoustid-flow', ['acoustid-form', 'acoustid-results', 'acoustid-review'], FINGERPRINT, async () => {
      await h.selectAll();
      await searchFlow('source-acoustid', { form: 'acoustid-form', results: 'acoustid-results', review: 'acoustid-review' });
    }),
    scene('fanart-flow', ['fanart-results'], REM, async () => {
      await h.selectAll();
      const d = await h.openDialog('source-cover-fanart', '#ts-search');
      d.querySelector('#ts-search').click();
      await h.waitFor(() => d.querySelectorAll('.ts-results tbody tr').length, 60000, 'fanart results');
      await h.sleep(1500);
      await h.snap('fanart-results', 'dialog');
      d.close();
    }),
    // Compare metadata, album mode. Takes: the window as opened, the sources panel, the window
    // after searching, one field's provider menu, and the review step. Never presses Save.
    scene('compare-flow', ['compare-start', 'compare-sources', 'compare-results', 'compare-use-menu', 'compare-review'], REM, async () => {
      await h.selectAll();
      const d = await h.openDialog('source-compare', '[data-fields]');
      await h.waitFor(() => !/Reading existing/i.test(d.querySelector('[data-status]')?.textContent || ''), 20000, 'snapshot read');
      await h.sleep(1200);
      await h.snap('compare-start', 'dialog');
      d.querySelector('[data-sources]').click(); await h.sleep(1500);
      await h.snap('compare-sources', 'dialog');
      // Close the sources panel again (its Back button), then search.
      const back = [...document.querySelectorAll('dialog[open] [data-close]')].at(-1);
      if (document.querySelectorAll('dialog[open]').length > 1 || back?.textContent.includes('Back')) back?.click();
      await h.sleep(600);
      const search = d.querySelector('[data-search]');
      await h.waitFor(() => !search.disabled, 10000, 'search enabled');
      search.click();
      await h.sleep(1500);
      await h.waitFor(() => { const stop = d.querySelector('[data-stop]'); return !stop || stop.hidden; }, 150000, 'sources finished');
      await h.sleep(2500);
      await h.report('LOG compare status: ' + (d.querySelector('[data-status]')?.textContent || ''));
      await h.snap('compare-results', 'dialog');
      const pick = d.querySelector('[data-pick-field="ALBUM"]') || d.querySelector('[data-pick-field]');
      if (pick) {
        pick.click(); await h.sleep(800);
        const menu = document.querySelector('[role="menu"], .mc-choice-menu, .helper-menu, [data-comparison-menu]');
        await h.report('LOG use menu: ' + (menu ? menu.textContent.trim().replace(/\s+/g, ' ').slice(0, 200) : 'none found'));
        await h.snap('compare-use-menu', 'dialog');
        document.body.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
        await h.sleep(300);
      }
      const review = d.querySelector('[data-review]');
      await h.report('LOG review enabled=' + (review && !review.disabled));
      d.close();
    }),
  ];
});
