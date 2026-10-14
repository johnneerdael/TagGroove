package org.opentag.fixture.documents;

import android.app.Activity;
import android.app.Instrumentation;
import android.content.ContentResolver;
import android.database.Cursor;
import android.net.Uri;
import android.os.Bundle;
import android.provider.DocumentsContract;
import java.io.*;
import java.nio.ByteBuffer;
import java.nio.ByteOrder;
import java.nio.charset.StandardCharsets;
import java.util.Arrays;

/** Pure ContentResolver checks; does not launch or automate a UI. Consumes disposable fixtures. */
public final class ProviderCheck extends Instrumentation {
    private int checks;
    @Override public void onCreate(Bundle args) { super.onCreate(args); start(); }
    private void check(boolean value, String why) { if (!value) throw new AssertionError(why); checks++; }
    private Uri uri(String id) { return DocumentsContract.buildDocumentUri(FixtureProvider.AUTHORITY, id); }
    private byte[] read(ContentResolver resolver, String id) throws Exception {
        try (InputStream in = resolver.openInputStream(uri(id)); ByteArrayOutputStream out = new ByteArrayOutputStream()) { FixtureProvider.transfer(in, out, Long.MAX_VALUE); return out.toByteArray(); }
    }
    private byte[] pcm(byte[] bytes) throws IOException {
        for (int offset = 12; offset + 8 <= bytes.length;) {
            int size = ByteBuffer.wrap(bytes, offset+4, 4).order(ByteOrder.LITTLE_ENDIAN).getInt();
            if (size < 0 || size > bytes.length-offset-8) throw new IOException("Malformed RIFF fixture");
            if (new String(bytes, offset, 4, StandardCharsets.US_ASCII).equals("data")) return Arrays.copyOfRange(bytes, offset+8, offset+8+size);
            offset += 8 + size + size%2;
        }
        throw new IOException("No PCM chunk");
    }
    @Override public void onStart() {
        Bundle result = new Bundle();
        try {
            ContentResolver resolver = getTargetContext().getContentResolver();
            try (Cursor roots = resolver.query(DocumentsContract.buildRootsUri(FixtureProvider.AUTHORITY), null, null, null, null)) { check(roots != null && roots.getCount() == 1, "Root visible"); }
            try (Cursor docs = resolver.query(DocumentsContract.buildChildDocumentsUri(FixtureProvider.AUTHORITY, FixtureProvider.ROOT), null, null, null, null)) { check(docs != null && docs.getCount() == 4, "Four seed fixtures"); }
            byte[] seed = read(resolver, "writable"); check(pcm(seed).length == 88200, "One second PCM");
            check(new String(seed, StandardCharsets.ISO_8859_1).contains("Fixture Original"), "Initial title");
            byte[] readonly = read(resolver, "readonly"); boolean refused = false;
            try (OutputStream unexpected = resolver.openOutputStream(uri("readonly"), "wt")) { unexpected.write(1); } catch (FileNotFoundException expected) { refused = true; }
            check(refused && Arrays.equals(readonly, read(resolver, "readonly")), "Read-only rejects before mutation");
            File conflict = FixtureProvider.file(getTargetContext(), "conflict"); long timestamp = conflict.lastModified(); byte[] before = read(resolver, "conflict");
            FixtureProvider.changeOutside(getTargetContext()); byte[] after = read(resolver, "conflict");
            check(before.length == after.length && conflict.lastModified() == timestamp, "Conflict preserves size/time");
            check(!Arrays.equals(before, after) && Arrays.equals(pcm(before), pcm(after)), "Conflict changes metadata only");
            FixtureProvider.prefs(getTargetContext()).edit().putBoolean("partial_next", true).commit();
            try (OutputStream output = resolver.openOutputStream(uri("partial"), "wt")) { output.write(seed); }
            check(read(resolver, "partial").length == 128, "Partial provider drains but retains only128bytes");
            Uri copy = DocumentsContract.createDocument(resolver, uri(FixtureProvider.ROOT), "audio/x-wav", "Saved Café 猫.wav"); check(copy != null, "Create destination");
            try (OutputStream output = resolver.openOutputStream(copy, "wt")) { output.write(seed); }
            try (InputStream input = resolver.openInputStream(copy); ByteArrayOutputStream output = new ByteArrayOutputStream()) { FixtureProvider.transfer(input, output, Long.MAX_VALUE); check(Arrays.equals(seed, output.toByteArray()), "Created copy exact bytes"); }
            result.putString("stream", "PASS " + checks + " ContentResolver fixture checks; generated documents consumed; clear this test package before UI UAT\n");
            finish(Activity.RESULT_OK, result);
        } catch (Throwable error) { result.putString("stream", "FAIL after " + checks + " checks: " + android.util.Log.getStackTraceString(error)); finish(Activity.RESULT_CANCELED, result); }
    }
}
