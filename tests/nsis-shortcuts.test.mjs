import test from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {execFileSync, spawnSync} from 'node:child_process';
import {mkdtemp, mkdir, readFile, rm, writeFile} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';

const reviewedWindows = process.platform === 'win32' && process.env.GITHUB_ACTIONS === 'true'
  && /\.github\/workflows\/(?:reviewed-(?:pr|release)|_reviewed-build)\.yml@refs\/heads\/main$/.test(process.env.GITHUB_WORKFLOW_REF || '');
const psQuote = value => `'${value.replaceAll("'", "''")}'`;
const nsisEscape = value => value.replaceAll('$', () => '$$').replaceAll('"', '$\\"');
const redirectFolders = source => source.replaceAll('$SMPROGRAMS', '$INSTDIR\\menu').replaceAll('$DESKTOP', '$INSTDIR\\desktop');

function extract(source, pattern, description) {
  const match = source.match(pattern);
  assert.ok(match, `Missing production ${description}`);
  return redirectFolders(match[0]);
}

test('NSIS shortcut fixtures compile and preserve owned links on Windows', {timeout: 180000}, async t => {
  const root = await mkdtemp(join(tmpdir(), 'taggroove-nsis-fixtures-'));
  t.after(() => rm(root, {recursive: true, force: true, maxRetries: 5, retryDelay: 200}));
  const upstream = JSON.parse(await readFile(new URL('../src-tauri/windows/UPSTREAM.json', import.meta.url)));
  const compilerEnvironment = Object.fromEntries(Object.entries(process.env)
    .filter(([name]) => !['NSISDIR', 'NSISCONFDIR'].includes(name.toUpperCase())));
  let compiler = process.platform === 'win32' ? 'makensis.exe' : 'makensis';
  if (reviewedWindows) {
    const toolset = upstream.testToolset;
    const url = new URL(toolset.url);
    assert.equal(url.origin, 'https://github.com');
    assert.ok(url.pathname.startsWith('/tauri-apps/binary-releases/releases/download/nsis-'));
    const response = await fetch(url, {signal: AbortSignal.timeout(60000)});
    assert.ok(response.ok, `NSIS toolset download returned ${response.status}`);
    const archive = Buffer.from(await response.arrayBuffer());
    assert.equal(createHash('sha256').update(archive).digest('hex'), toolset.sha256);
    const zip = join(root, 'nsis.zip'), tools = join(root, 'tools');
    await writeFile(zip, archive);
    execFileSync('powershell.exe', ['-NoProfile', '-NonInteractive', '-Command',
      `Expand-Archive -LiteralPath ${psQuote(zip)} -DestinationPath ${psQuote(tools)}`], {timeout: 60000});
    compiler = join(tools, `nsis-${toolset.version}`, 'makensis.exe');
  } else if (spawnSync(compiler, ['-VERSION'], {env: compilerEnvironment}).status !== 0) {
    t.skip('Local compile check requires makensis; reviewed Windows CI supplies the pinned compiler');
    return;
  }

  const source = await readFile(new URL('../src-tauri/windows/installer.nsi', import.meta.url), 'utf8');
  const utilities = await readFile(new URL('../' + upstream.shortcutUtilities.fixturePath, import.meta.url));
  assert.equal(createHash('sha256').update(utilities).digest('hex'), upstream.shortcutUtilities.fixtureSha256);
  const fixture = await readFile(new URL('./fixtures/nsis-shortcuts.nsi', import.meta.url), 'utf8');
  const parts = {
    UTILITY_MACROS: utilities.toString('utf8'),
    MIGRATION_MACRO: extract(source, /^!macro MigrateLegacyShortcut[^\n]*\n[\s\S]*?^!macroend/m, 'migration macro'),
    CLEANUP_MACRO: extract(source, /^!macro RemoveLegacyShortcut[^\n]*\n[\s\S]*?^!macroend/m, 'cleanup macro'),
    START_MENU_FUNCTION: extract(source, /^Function CreateOrUpdateStartMenuShortcut\n[\s\S]*?^FunctionEnd/m, 'Start Menu function'),
    DESKTOP_FUNCTION: extract(source, /^Function CreateOrUpdateDesktopShortcut\n[\s\S]*?^FunctionEnd/m, 'desktop function'),
    INSTALL_MIGRATION: extract(source, /^  ; Existing links are upgrades,[\s\S]*?(?=^  ; Create start menu shortcut)/m, 'install migration block'),
    UNINSTALL_CLEANUP: extract(source, /^  ; Remove retained legacy labels[\s\S]*?^  ; End retained legacy labels\./m, 'uninstall cleanup block'),
  };
  for (const [name, folder] of [['flat', ''], ['folder', 'FixtureFolder']]) {
    const directory = join(root, name);
    await mkdir(directory);
    const binary = join(directory, 'shortcuts.exe'), script = join(directory, 'shortcuts.nsi');
    const values = {...parts, OUTFILE: nsisEscape(binary), STARTMENUFOLDER: folder};
    const rendered = fixture.replace(/@([A-Z_]+)@/g, (_, key) => {
      assert.ok(Object.hasOwn(values, key), `Unknown fixture placeholder ${key}`);
      return values[key];
    });
    await writeFile(script, rendered);
    execFileSync(compiler, ['-INPUTCHARSET', 'UTF8', '-OUTPUTCHARSET', 'UTF8', '-V2', script],
      {timeout: 30000, stdio: 'pipe', env: compilerEnvironment, cwd: directory});
    if (reviewedWindows) {
      try {
        execFileSync(binary, ['/S'], {timeout: 30000, windowsHide: true, stdio: 'pipe'});
      } catch (error) {
        const failure = await readFile(join(directory, 'failure.txt'), 'utf8').catch(() => error.message);
        assert.fail(`${name} NSIS fixture failed: ${failure}`);
      }
      assert.equal((await readFile(join(directory, 'result.txt'), 'utf8')).trim(), 'ok');
    }
  }
  t.diagnostic(reviewedWindows
    ? 'Executed production migration, creation and cleanup with real COM shortcuts in isolated flat/folder fixtures'
    : 'Compiled both fixture layouts; native execution is reserved for reviewed Windows CI');
});
