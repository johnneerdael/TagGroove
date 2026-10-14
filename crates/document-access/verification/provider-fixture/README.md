# Disposable Android document provider

This separate test application exposes four generated one-second WAV documents
through the real Android Storage Access Framework. It is never packaged in
OpenTag and requests no broad storage or network permission. Its provider is
protected with `MANAGE_DOCUMENTS`; access from OpenTag must come from the system
picker and its URI grants.

Use only a dedicated disposable emulator. The fixture controls intentionally
change or truncate their own generated audio. They never access a user library.

- Writable Unicode name: `01 - Mémoire - Café 猫.wav`.
- Read-only document: no write capability and writes refused by the provider.
- Conflict document: a control replaces its title with another 16-byte title,
  preserving file size, modification time and the PCM data chunk.
- Interrupted document: arm a control before saving. A reliable pipe consumes
  the entire write but retains only its first 128 bytes; a subsequent read waits
  for the pipe worker. The application's independent read-back must detect this.
- Copy destinations: `createDocument` allocates distinct UUID identities and
  accepts streamed writes, including empty initial documents.

The control activity can also revoke the conflict document's URI grants and
make it unavailable. It provides no automatic restoration. Retained recovery
copies must remain accessible through OpenTag when the source is unavailable.
Fixture events and bytes live under this test application's private `files/`
directory, accessible for independent receipts through `run-as` in debug builds.

Build with Gradle 9.6.1, AGP 9.3.1, JDK 21 and an installed Android SDK 36.
Use an existing Gradle launcher; no additional wrapper binary is vendored:

```sh
gradle assembleDebug assembleDebugAndroidTest --offline --console=plain
```

Install both generated APKs on the dedicated emulator. Run the pure provider
checks before UI testing:

```sh
adb -s DEVICE shell am instrument -w org.opentag.fixture.documents.test/org.opentag.fixture.documents.ProviderCheck
```

Require an explicit `PASS` stream result. These checks consume generated test
documents. Clear **only** `org.opentag.fixture.documents` on that disposable
device before starting UI UAT, then open the fixture activity or query its root
to seed fresh documents. Clearing data invalidates previously issued fixture
URIs and must never be used as an OpenTag recovery procedure.

The provider follows Android's [custom-provider guide](https://developer.android.com/guide/topics/providers/create-document-provider)
and [DocumentsProvider contract](https://developer.android.com/reference/android/provider/DocumentsProvider).
These native API checks validate the fixture, not OpenTag's command routing,
picker lifecycle, save-back, recovery UI or device compatibility. Those require
separate app tests and independent file/audio receipts.
