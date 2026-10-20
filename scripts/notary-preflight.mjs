#!/usr/bin/env node
// Authenticate before release builds without submitting software or reading submission data.
import {createPrivateKey, sign} from 'node:crypto';
import {pathToFileURL} from 'node:url';

const ENDPOINT = 'https://appstoreconnect.apple.com/notary/v2/submissions';
const TIMEOUT_MS = 15_000;
const MAX_KEY_BYTES = 48 * 1024;
class PreflightError extends Error {}

function requireSafe(condition, message) {
  if (!condition) throw new PreflightError(message);
}

function privateKeyFromEnvironment(value) {
  requireSafe(typeof value === 'string' && value.length > 0 && value.length <= 96 * 1024,
    'Missing or invalid notary private key');
  let pem = value;
  if (!pem.startsWith('-----BEGIN PRIVATE KEY-----')) {
    const encoded = value.replace(/\s/g, '');
    requireSafe(encoded.length > 0 && encoded.length % 4 === 0 && /^[A-Za-z0-9+/]*={0,2}$/.test(encoded),
      'Invalid notary private-key encoding');
    const decoded = Buffer.from(encoded, 'base64');
    requireSafe(decoded.toString('base64') === encoded, 'Invalid notary private-key encoding');
    pem = decoded.toString('utf8');
  }
  requireSafe(Buffer.byteLength(pem) <= MAX_KEY_BYTES &&
    /^-----BEGIN PRIVATE KEY-----\r?\n[A-Za-z0-9+/=\r\n]+\r?\n-----END PRIVATE KEY-----\r?\n?$/.test(pem),
    'Invalid notary private-key envelope');
  let key;
  try {
    key = createPrivateKey({key: pem, format: 'pem', type: 'pkcs8'});
  } catch {
    throw new PreflightError('Invalid notary private key');
  }
  requireSafe(key.asymmetricKeyType === 'ec' && key.asymmetricKeyDetails?.namedCurve === 'prime256v1',
    'Notary private key must use P-256');
  return key;
}

/** Build a five-minute team-key token; never print the returned credential. */
export function createNotaryToken(environment, {now = Date.now} = {}) {
  try {
    const issuer = environment.APPLE_NOTARY_ISSUER_ID;
    const kid = environment.APPLE_NOTARY_KEY_ID;
    requireSafe(typeof issuer === 'string' && /^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$/i.test(issuer),
      'Missing or invalid notary issuer identifier');
    requireSafe(typeof kid === 'string' && /^[A-Z0-9]{10}$/.test(kid),
      'Missing or invalid notary key identifier');
    const key = privateKeyFromEnvironment(environment.APPLE_NOTARY_PRIVATE_KEY);
    const iat = Math.floor(now() / 1000);
    requireSafe(Number.isSafeInteger(iat) && iat > 0, 'Invalid credential token time');
    const encode = (value) => Buffer.from(JSON.stringify(value)).toString('base64url');
    const input = `${encode({alg: 'ES256', kid, typ: 'JWT'})}.${encode({iss: issuer, iat, exp: iat + 300, aud: 'appstoreconnect-v1'})}`;
    const signature = sign('sha256', Buffer.from(input), {key, dsaEncoding: 'ieee-p1363'});
    requireSafe(signature.length === 64, 'Unable to sign notary credential token');
    return `${input}.${signature.toString('base64url')}`;
  } catch (error) {
    if (error instanceof PreflightError) throw error;
    throw new PreflightError('Unable to prepare notary credential token');
  }
}

/** A single bounded, read-only authentication probe. Response bodies remain unread. */
export async function preflightNotary(environment, {fetchImpl = globalThis.fetch, now = Date.now, timeoutMs = TIMEOUT_MS} = {}) {
  const token = createNotaryToken(environment, {now});
  requireSafe(Number.isInteger(timeoutMs) && timeoutMs > 0 && timeoutMs <= TIMEOUT_MS,
    'Invalid notary request timeout');
  const controller = new AbortController();
  let timer;
  try {
    const timeout = new Promise((_, reject) => {
      timer = setTimeout(() => {
        reject(new PreflightError('Notary API request timed out'));
        controller.abort();
      }, timeoutMs);
    });
    const response = await Promise.race([
      Promise.resolve().then(() => fetchImpl(ENDPOINT, {
        method: 'GET', redirect: 'error', signal: controller.signal,
        headers: {Authorization: `Bearer ${token}`},
      })),
      timeout,
    ]);
    requireSafe(response?.redirected !== true, 'Notary API redirect rejected');
    const status = response?.status;
    requireSafe(Number.isInteger(status) && status >= 100 && status <= 599, 'Invalid Notary API response');
    requireSafe(status === 200, `Notary API authentication rejected (HTTP ${status})`);
    return {status: 200};
  } catch (error) {
    if (error instanceof PreflightError) throw error;
    throw new PreflightError('Notary API request failed');
  } finally {
    clearTimeout(timer);
    // Abort after headers, too: discard the stream without reading user data.
    controller.abort();
  }
}

export async function main({env = process.env, stdout = (text) => process.stdout.write(text),
  stderr = (text) => process.stderr.write(text), ...options} = {}) {
  try {
    await preflightNotary(env, options);
    stdout('Apple Notary credential preflight authenticated (HTTP 200).\n');
    return 0;
  } catch (error) {
    const message = error instanceof PreflightError ? error.message : 'Credential check failed';
    stderr(`Apple Notary credential preflight failed: ${message}.\n`);
    return 1;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  process.exitCode = await main();
}
