package com.stephenshorton.irori

import android.os.Bundle
import android.webkit.JavascriptInterface
import android.webkit.WebView
import androidx.activity.enableEdgeToEdge
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat

class MainActivity : TauriActivity() {
  // The system bars' insets in CSS px (card #395). The page draws under the
  // status and navigation bars (edge to edge); a WebView before 136 reads
  // env(safe-area-inset-*) as 0, so the page reads these too
  // (window.IroriInsets, frontend/src/main.tsx).
  @Volatile private var top = 0f
  @Volatile private var bottom = 0f
  @Volatile private var left = 0f
  @Volatile private var right = 0f

  override fun onCreate(savedInstanceState: Bundle?) {
    enableEdgeToEdge()
    super.onCreate(savedInstanceState)
  }

  override fun onWebViewCreate(webView: WebView) {
    webView.addJavascriptInterface(Insets(), "IroriInsets")
    ViewCompat.setOnApplyWindowInsetsListener(webView) { v, insets ->
      val b = insets.getInsets(WindowInsetsCompat.Type.systemBars() or WindowInsetsCompat.Type.displayCutout())
      val d = resources.displayMetrics.density
      top = b.top / d
      bottom = b.bottom / d
      left = b.left / d
      right = b.right / d
      v.post { (v as WebView).evaluateJavascript("window.dispatchEvent(new Event('irori:insets'))", null) }
      insets
    }
  }

  inner class Insets {
    @JavascriptInterface fun top(): Float = top
    @JavascriptInterface fun bottom(): Float = bottom
    @JavascriptInterface fun left(): Float = left
    @JavascriptInterface fun right(): Float = right
  }
}
