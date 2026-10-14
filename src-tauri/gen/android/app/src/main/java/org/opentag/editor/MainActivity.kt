package org.opentag.editor

import android.os.Bundle
import android.view.View
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import androidx.core.view.doOnAttach

class MainActivity : TauriActivity() {
  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    super.onWebViewCreate(webView)
    // Wry calls this hook before setContentView, so wait for the existing parent.
    webView.doOnAttach {
      val content = webView.parent as View
      val left = content.paddingLeft
      val top = content.paddingTop
      val right = content.paddingRight
      val bottom = content.paddingBottom
      val safeTypes = WindowInsetsCompat.Type.systemBars() or
        WindowInsetsCompat.Type.displayCutout()
      ViewCompat.setOnApplyWindowInsetsListener(content) { view, insets ->
        val safe = insets.getInsets(safeTypes or WindowInsetsCompat.Type.ime())
        // Inset the parent so the measured WebView (and CSS viewport) both shrink.
        view.setPadding(left + safe.left, top + safe.top, right + safe.right, bottom + safe.bottom)
        // The viewport already avoids these areas. Preserve other types, including IME.
        WindowInsetsCompat.Builder(insets)
          .setInsets(safeTypes, Insets.NONE)
          .setDisplayCutout(null)
          .build()
      }
      ViewCompat.requestApplyInsets(content)
    }
  }
}
