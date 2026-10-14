package org.opentag.fixture.documents;

import android.content.Context;
import android.content.SharedPreferences;
import android.database.Cursor;
import android.database.MatrixCursor;
import android.net.Uri;
import android.os.CancellationSignal;
import android.os.ParcelFileDescriptor;
import android.provider.DocumentsContract;
import android.provider.DocumentsProvider;
import java.io.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.security.MessageDigest;
import java.util.*;
import java.util.concurrent.*;
import org.json.JSONObject;

/** Disposable SAF test provider. Never included in the OpenTag application. */
public final class FixtureProvider extends DocumentsProvider {
    static final String AUTHORITY = "org.opentag.fixture.documents.provider";
    static final String ROOT = "root";
    static final String[] IDS = { "writable", "readonly", "conflict", "partial" };
    static final String[] NAMES = { "01 - Mémoire - Café 猫.wav", "02 - Read only.wav", "03 - Changed outside.wav", "04 - Interrupted write.wav" };
    private static final String[] DOC_COLUMNS = { "document_id", "_display_name", "mime_type", "flags", "_size", "last_modified" };
    private static final String[] ROOT_COLUMNS = { "root_id", "document_id", "title", "summary", "flags", "mime_types", "icon" };
    private static final ConcurrentHashMap<String, CountDownLatch> WRITES = new ConcurrentHashMap<>();
    private static final ExecutorService STREAMS = Executors.newSingleThreadExecutor();

    static SharedPreferences prefs(Context c) { return c.getSharedPreferences("fixture", Context.MODE_PRIVATE); }
    static File directory(Context c) { return new File(c.getFilesDir(), "documents"); }
    static File file(Context c, String id) throws FileNotFoundException {
        if (!id.matches("writable|readonly|conflict|partial|copy-[0-9a-f-]{36}")) throw new FileNotFoundException("Unknown fixture identity");
        return new File(directory(c), id + ".wav");
    }
    static synchronized void seed(Context c) throws IOException {
        File dir = directory(c);
        if (!dir.isDirectory() && !dir.mkdirs()) throw new IOException("Cannot create fixture directory");
        for (String id : IDS) {
            File f = file(c, id);
            if (!f.exists()) try (InputStream in = c.getAssets().open("seed.wav"); FileOutputStream out = new FileOutputStream(f)) {
                transfer(in, out, Long.MAX_VALUE); out.getFD().sync();
            }
        }
    }
    static long transfer(InputStream in, OutputStream out, long limit) throws IOException {
        byte[] buffer = new byte[8192]; long count = 0; int n;
        while ((n = in.read(buffer)) != -1) { int written = (int)Math.min(n, Math.max(0, limit - count)); if (written > 0) out.write(buffer, 0, written); count += n; }
        return count;
    }
    static String sha(File file) throws Exception {
        MessageDigest hash = MessageDigest.getInstance("SHA-256");
        try (InputStream in = new FileInputStream(file)) { byte[] b = new byte[8192]; int n; while ((n = in.read(b)) != -1) hash.update(b, 0, n); }
        StringBuilder result = new StringBuilder(); for (byte b : hash.digest()) result.append(String.format(Locale.ROOT, "%02x", b)); return result.toString();
    }
    static synchronized void event(Context c, String action, String id, String detail) {
        try (FileWriter out = new FileWriter(new File(c.getFilesDir(), "events.jsonl"), true)) {
            out.write(new JSONObject().put("time_ms", System.currentTimeMillis()).put("action", action).put("id", id).put("detail", detail).toString() + "\n");
        } catch (Exception e) { throw new IllegalStateException("Fixture receipt failed", e); }
    }
    static synchronized void changeOutside(Context c) throws Exception {
        File f = file(c, "conflict"); long timestamp = f.lastModified(); byte[] bytes = Files.readAllBytes(f.toPath());
        byte[] from = "Fixture Original".getBytes(StandardCharsets.UTF_8), to = "Outside Modified".getBytes(StandardCharsets.UTF_8);
        int offset = -1;
        for (int i = 0; i <= bytes.length - from.length; i++) { boolean same = true; for (int j = 0; j < from.length; j++) if (bytes[i+j] != from[j]) { same = false; break; } if (same) { offset = i; break; } }
        if (offset < 0) throw new IOException("Expected original title absent; reseed the fixture first");
        System.arraycopy(to, 0, bytes, offset, to.length);
        try (FileOutputStream out = new FileOutputStream(f)) { out.write(bytes); out.getFD().sync(); }
        if (!f.setLastModified(timestamp)) throw new IOException("Cannot preserve fixture timestamp");
        event(c, "outside_change", "conflict", sha(f));
    }
    static void revoke(Context c) {
        Uri uri = DocumentsContract.buildDocumentUri(AUTHORITY, "conflict");
        c.revokeUriPermission(uri, android.content.Intent.FLAG_GRANT_READ_URI_PERMISSION | android.content.Intent.FLAG_GRANT_WRITE_URI_PERMISSION);
        prefs(c).edit().putBoolean("unavailable", true).commit();
        event(c, "revoke", "conflict", "Grant revoked and source made unavailable");
    }
    static void notifyRoot(Context c) { c.getContentResolver().notifyChange(DocumentsContract.buildChildDocumentsUri(AUTHORITY, ROOT), null); }
    private void available(String id) throws FileNotFoundException {
        if (id.equals("conflict") && prefs(getContext()).getBoolean("unavailable", false)) throw new FileNotFoundException("Fixture source unavailable");
    }
    @Override public boolean onCreate() { try { seed(getContext()); return true; } catch (IOException e) { throw new IllegalStateException(e); } }
    @Override public Cursor queryRoots(String[] projection) {
        MatrixCursor cursor = new MatrixCursor(projection == null ? ROOT_COLUMNS : projection);
        Map<String,Object> row = new HashMap<>(); row.put("root_id", ROOT); row.put("document_id", ROOT); row.put("title", "OpenTag Test Documents"); row.put("summary", "Disposable generated WAV fixtures"); row.put("flags", DocumentsContract.Root.FLAG_SUPPORTS_CREATE); row.put("mime_types", "audio/wav\naudio/x-wav"); row.put("icon", android.R.drawable.ic_menu_save);
        append(cursor, row); return cursor;
    }
    private static void append(MatrixCursor cursor, Map<String,Object> values) { MatrixCursor.RowBuilder row = cursor.newRow(); for (String column : cursor.getColumnNames()) row.add(values.get(column)); }
    private void addDocument(MatrixCursor cursor, String id) throws FileNotFoundException {
        Map<String,Object> row = new HashMap<>(); row.put("document_id", id);
        if (id.equals(ROOT)) { row.put("_display_name", "OpenTag Test Documents"); row.put("mime_type", DocumentsContract.Document.MIME_TYPE_DIR); row.put("flags", DocumentsContract.Document.FLAG_DIR_SUPPORTS_CREATE); }
        else {
            available(id); File f = file(getContext(), id); if (!f.isFile()) throw new FileNotFoundException("Missing fixture");
            int i = Arrays.asList(IDS).indexOf(id); row.put("_display_name", i >= 0 ? NAMES[i] : prefs(getContext()).getString(id, id + ".wav"));
            row.put("mime_type", "audio/x-wav"); row.put("flags", id.equals("readonly") ? 0 : DocumentsContract.Document.FLAG_SUPPORTS_WRITE); row.put("_size", f.length()); row.put("last_modified", f.lastModified());
        }
        append(cursor, row);
    }
    @Override public Cursor queryDocument(String id, String[] projection) throws FileNotFoundException { MatrixCursor cursor = new MatrixCursor(projection == null ? DOC_COLUMNS : projection); addDocument(cursor, id); return cursor; }
    @Override public Cursor queryChildDocuments(String id, String[] projection, String sort) throws FileNotFoundException {
        if (!id.equals(ROOT)) throw new FileNotFoundException("Not a directory"); MatrixCursor cursor = new MatrixCursor(projection == null ? DOC_COLUMNS : projection);
        for (String child : IDS) { if (child.equals("conflict") && prefs(getContext()).getBoolean("unavailable", false)) continue; addDocument(cursor, child); }
        File[] copies = directory(getContext()).listFiles((dir, name) -> name.startsWith("copy-") && name.endsWith(".wav"));
        if (copies != null) { Arrays.sort(copies); for (File f : copies) addDocument(cursor, f.getName().substring(0, f.getName().length()-4)); }
        return cursor;
    }
    @Override public String createDocument(String parent, String mime, String name) throws FileNotFoundException {
        if (!parent.equals(ROOT)) throw new FileNotFoundException("Not a directory");
        if (name == null || name.isEmpty() || name.contains("/") || name.contains("\\") || name.indexOf(0) >= 0) throw new FileNotFoundException("Invalid copy name");
        String id = "copy-" + UUID.randomUUID();
        try { if (!file(getContext(), id).createNewFile()) throw new IOException("Copy exists"); prefs(getContext()).edit().putString(id, name).commit(); event(getContext(), "create", id, name); notifyRoot(getContext()); return id; }
        catch (IOException e) { throw new FileNotFoundException(e.toString()); }
    }
    @Override public ParcelFileDescriptor openDocument(String id, String mode, CancellationSignal signal) throws FileNotFoundException {
        available(id); File f = file(getContext(), id); boolean write = mode.indexOf('w') >= 0;
        if (write && id.equals("readonly")) throw new FileNotFoundException("Read-only fixture");
        try {
            if (!write) { CountDownLatch pending = WRITES.get(id); if (pending != null && !pending.await(10, TimeUnit.SECONDS)) throw new IOException("Fixture write timeout"); }
            event(getContext(), write ? "write_open" : "read_open", id, mode + ":" + sha(f));
            boolean partialFixture = id.equals("partial") && prefs(getContext()).getBoolean("partial_next", false);
            boolean partialCopy = id.startsWith("copy-") && prefs(getContext()).getBoolean("copy_partial_next", false);
            if (write && (partialFixture || partialCopy)) {
                if (partialFixture) prefs(getContext()).edit().putBoolean("partial_next", false).commit();
                if (partialCopy) prefs(getContext()).edit().putBoolean("copy_partial_next", false).commit();
                ParcelFileDescriptor[] pipe = ParcelFileDescriptor.createReliablePipe(); CountDownLatch complete = new CountDownLatch(1); WRITES.put(id, complete);
                STREAMS.submit(() -> {
                    try (InputStream input = new ParcelFileDescriptor.AutoCloseInputStream(pipe[0]); FileOutputStream output = new FileOutputStream(f, false)) {
                        long received = transfer(input, output, 128); output.getFD().sync(); event(getContext(), "partial_write", id, "received=" + received + ",retained=" + f.length());
                    } catch (Exception e) { event(getContext(), "partial_error", id, e.toString()); }
                    finally { complete.countDown(); WRITES.remove(id, complete); }
                });
                return pipe[1];
            }
            return ParcelFileDescriptor.open(f, ParcelFileDescriptor.parseMode(mode));
        } catch (Exception e) { throw new FileNotFoundException(e.toString()); }
    }
}
