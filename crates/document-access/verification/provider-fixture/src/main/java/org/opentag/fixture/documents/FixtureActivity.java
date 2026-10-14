package org.opentag.fixture.documents;

import android.app.Activity;
import android.os.Bundle;
import android.view.ViewGroup;
import android.widget.*;

public final class FixtureActivity extends Activity {
    private TextView status;
    interface Operation { void run() throws Exception; }
    private void button(LinearLayout layout, String label, Operation work) {
        Button button = new Button(this); button.setText(label); layout.addView(button);
        button.setOnClickListener(v -> { try { work.run(); status.setText(label + " completed\n" + summary()); } catch (Exception e) { status.setText(e.toString()); } });
    }
    private String summary() {
        return "Generated WAV files only.\nNext partial write armed: " + FixtureProvider.prefs(this).getBoolean("partial_next", false)
            + "\nConflict source unavailable: " + FixtureProvider.prefs(this).getBoolean("unavailable", false);
    }
    @Override public void onCreate(Bundle saved) {
        super.onCreate(saved);
        LinearLayout layout = new LinearLayout(this); layout.setOrientation(LinearLayout.VERTICAL); layout.setPadding(24, 24, 24, 24);
        status = new TextView(this); status.setText("OpenTag SAF test provider\n" + summary()); status.setTextSize(18); layout.addView(status);
        button(layout, "Change conflict source metadata", () -> { FixtureProvider.changeOutside(this); FixtureProvider.notifyRoot(this); });
        button(layout, "Arm partial write for interrupted fixture", () -> FixtureProvider.prefs(this).edit().putBoolean("partial_next", true).commit());
        button(layout, "Revoke conflict source access", () -> { FixtureProvider.revoke(this); FixtureProvider.notifyRoot(this); });
        TextView note = new TextView(this); note.setText("Choose these files through OpenTag > Add files > OpenTag Test Documents.\nThe read-only fixture rejects writes. Partial injection drains the edited stream but retains only 128 bytes. Recovery must preserve both copies.\nReseed by clearing ONLY this test application's data on the dedicated AVD; that invalidates previous fixture URIs."); layout.addView(note);
        ScrollView scroll = new ScrollView(this); scroll.addView(layout, new ViewGroup.LayoutParams(-1, -2)); setContentView(scroll);
    }
}
