import test from 'node:test';
import assert from 'node:assert/strict';
import {generateKeyPairSync, verify} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {createNotaryToken, preflightNotary, main} from '../scripts/notary-preflight.mjs';

const {privateKey, publicKey} = generateKeyPairSync('ec', {namedCurve: 'prime256v1'});
const pem = privateKey.export({type: 'pkcs8', format: 'pem'});
const credentials = {
  APPLE_NOTARY_ISSUER_ID: '12345678-1234-1234-1234-123456789abc',
  APPLE_NOTARY_KEY_ID: 'ABCDEFGHIJ',
  APPLE_NOTARY_PRIVATE_KEY: pem,
};
const now = () => 1_800_000_000_999;

test('signs a short-lived ES256 JWT with an independently verifiable raw P-256 signature', () => {
  const token = createNotaryToken(credentials, {now});
  const [header, claims, signature] = token.split('.');
  assert.deepEqual(JSON.parse(Buffer.from(header, 'base64url')), {alg: 'ES256', kid: 'ABCDEFGHIJ', typ: 'JWT'});
  assert.deepEqual(JSON.parse(Buffer.from(claims, 'base64url')), {
    iss: '12345678-1234-1234-1234-123456789abc', iat: 1_800_000_000,
    exp: 1_800_000_300, aud: 'appstoreconnect-v1',
  });
  const raw = Buffer.from(signature, 'base64url');
  assert.equal(raw.length, 64);
  assert.equal(verify('sha256', Buffer.from(`${header}.${claims}`), {key: publicKey, dsaEncoding: 'ieee-p1363'}, raw), true);
});

test('accepts whitespace-wrapped base64 PKCS8 PEM without changing credentials', () => {
  const encoded = Buffer.from(pem).toString('base64').match(/.{1,48}/g).join('\n');
  const token = createNotaryToken({...credentials, APPLE_NOTARY_PRIVATE_KEY: encoded}, {now});
  const [header, claims, signature] = token.split('.');
  assert.equal(verify('sha256', Buffer.from(`${header}.${claims}`), {key: publicKey, dsaEncoding: 'ieee-p1363'}, Buffer.from(signature, 'base64url')), true);
});

test('rejects missing and malformed credential identifiers before making any request', async () => {
  for (const patch of [
    {APPLE_NOTARY_ISSUER_ID: ''}, {APPLE_NOTARY_ISSUER_ID: 'not-a-uuid'},
    {APPLE_NOTARY_ISSUER_ID: `${credentials.APPLE_NOTARY_ISSUER_ID}\n`},
    {APPLE_NOTARY_KEY_ID: ''}, {APPLE_NOTARY_KEY_ID: 'abcde12345'},
    {APPLE_NOTARY_KEY_ID: 'ABCDEFGHIJ\n'}, {APPLE_NOTARY_PRIVATE_KEY: ''},
  ]) {
    let requested = false;
    await assert.rejects(preflightNotary({...credentials, ...patch}, {
      now, fetchImpl: async () => {requested = true; throw new Error('unexpected request');},
    }));
    assert.equal(requested, false);
  }
});

test('rejects malformed encoding, oversized material, wrong envelopes and non-P256 keys', () => {
  const other = generateKeyPairSync('ec', {namedCurve: 'secp384r1'}).privateKey;
  const rsa = generateKeyPairSync('rsa', {modulusLength: 2048}).privateKey;
  for (const key of [
    'secret-looking-invalid-key', Buffer.from(pem).toString('base64') + '!','x'.repeat(100_000),
    pem.replace('PRIVATE KEY', 'EC PRIVATE KEY'), '-----BEGIN PRIVATE KEY-----\nAAAA\n-----END PRIVATE KEY-----\n',
    other.export({type: 'pkcs8', format: 'pem'}), rsa.export({type: 'pkcs8', format: 'pem'}),
    privateKey.export({type: 'sec1', format: 'pem'}), publicKey.export({type: 'spki', format: 'pem'}),
  ]) {
    assert.throws(() => createNotaryToken({...credentials, APPLE_NOTARY_PRIVATE_KEY: key}, {now}),
      (error) => !error.message.includes(key) && !error.message.includes('secret-looking'));
  }
});

test('makes exactly one fixed authenticated GET, rejects redirects and never reads a response body', async () => {
  let calls = 0;
  let requestSignal;
  const result = await preflightNotary(credentials, {now, fetchImpl: async (url, options) => {
    calls++;
    assert.equal(url, 'https://appstoreconnect.apple.com/notary/v2/submissions');
    assert.equal(options.method, 'GET');
    assert.equal(options.redirect, 'error');
    assert.equal(options.body, undefined);
    assert.deepEqual(Object.keys(options.headers), ['Authorization']);
    assert.match(options.headers.Authorization, /^Bearer [\w-]+\.[\w-]+\.[\w-]+$/);
    const [header, claims, signature] = options.headers.Authorization.slice(7).split('.');
    assert.equal(JSON.parse(Buffer.from(header, 'base64url')).kid, 'ABCDEFGHIJ');
    assert.equal(JSON.parse(Buffer.from(claims, 'base64url')).iss, '12345678-1234-1234-1234-123456789abc');
    assert.equal(verify('sha256', Buffer.from(`${header}.${claims}`), {key: publicKey, dsaEncoding: 'ieee-p1363'}, Buffer.from(signature, 'base64url')), true);
    requestSignal = options.signal;
    assert.equal(requestSignal.aborted, false);
    return {status: 200, redirected: false,
      get body() {assert.fail('must not inspect submission data');},
      text() {assert.fail('must not read submission data');},
      json() {assert.fail('must not parse submission data');},
    };
  }});
  assert.deepEqual(result, {status: 200});
  assert.equal(calls, 1);
  assert.equal(requestSignal.aborted, true, 'close the unused response stream');
});

test('rejects every non-200 response with only a numeric status', async () => {
  for (const status of [201, 204, 301, 302, 307, 308, 400, 401, 403, 429, 500, 503]) {
    await assert.rejects(preflightNotary(credentials, {now, fetchImpl: async () => ({status,
      statusText: 'secret-looking-status', text: () => {throw new Error('secret-looking-body');},
    })}), (error) => error.message.includes(`HTTP ${status}`) && !error.message.includes('secret-looking'));
  }
});

test('rejects redirected success responses and malformed status without leaking response fields', async () => {
  for (const response of [{status: 200, redirected: true}, {status: 'secret-looking-status'}, {status: 999}]) {
    await assert.rejects(preflightNotary(credentials, {now, fetchImpl: async () => response}),
      (error) => !error.message.includes('secret-looking'));
  }
});

test('sanitizes network and clock exceptions containing secrets or tokens', async () => {
  const secret = 'secret-looking-token-and-private-key';
  await assert.rejects(preflightNotary(credentials, {now, fetchImpl: async () => {throw new Error(secret);}}),
    (error) => !String(error.stack).includes(secret));
  assert.throws(() => createNotaryToken(credentials, {now: () => {throw new Error(secret);}}),
    (error) => !String(error.stack).includes(secret));
});

test('bounds a stalled request even when the transport ignores abort', async () => {
  let signal;
  const started = Date.now();
  await assert.rejects(preflightNotary(credentials, {now, timeoutMs: 15, fetchImpl: async (_url, options) => {
    signal = options.signal;
    return new Promise(() => {});
  }}), /timed out/);
  assert.ok(Date.now() - started < 2000);
  assert.equal(signal.aborted, true);
});

test('CLI entry reports only authenticated status on success and sanitized failures', async () => {
  for (const scenario of ['success', 'http', 'network', 'key']) {
    let stdout = '', stderr = '';
    const code = await main({
      env: scenario === 'key' ? {...credentials, APPLE_NOTARY_PRIVATE_KEY: 'secret-looking-key'} : credentials,
      now, stdout: (text) => {stdout += text;}, stderr: (text) => {stderr += text;},
      fetchImpl: async () => {
        if (scenario === 'network') throw new Error('secret-looking-network');
        return {status: scenario === 'http' ? 401 : 200};
      },
    });
    assert.equal(code, scenario === 'success' ? 0 : 1);
    assert.equal(stdout, scenario === 'success' ? 'Apple Notary credential preflight authenticated (HTTP 200).\n' : '');
    assert.equal(stderr === '', scenario === 'success');
    assert.equal((stdout + stderr).includes('secret-looking'), false);
    assert.equal((stdout + stderr).includes(credentials.APPLE_NOTARY_ISSUER_ID), false);
    assert.equal((stdout + stderr).includes(credentials.APPLE_NOTARY_KEY_ID), false);
    assert.equal((stdout + stderr).includes('BEGIN PRIVATE KEY'), false);
    assert.doesNotMatch(stdout + stderr, /eyJ[\w-]+\.[\w-]+\.[\w-]+/);
  }
});

test('direct executable invocation rejects absent credentials without raw exceptions or network access', () => {
  const child = spawnSync(process.execPath, ['scripts/notary-preflight.mjs'], {encoding: 'utf8', env: {PATH: process.env.PATH}});
  assert.equal(child.status, 1);
  assert.equal(child.stdout, '');
  assert.match(child.stderr, /^Apple Notary credential preflight failed: [^\n]+\n$/);
  assert.doesNotMatch(child.stderr, /Error:| at |BEGIN PRIVATE KEY/);
});
