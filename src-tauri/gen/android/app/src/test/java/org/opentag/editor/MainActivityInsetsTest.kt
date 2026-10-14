package org.opentag.editor

import android.app.Activity
import android.view.View
import android.view.ViewGroup
import android.webkit.WebView
import androidx.core.graphics.Insets
import androidx.core.view.ViewCompat
import androidx.core.view.WindowInsetsCompat
import org.junit.Assert.assertEquals
import org.junit.Assert.assertSame
import org.junit.Assert.assertTrue
import org.junit.After
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.Robolectric
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import org.robolectric.android.controller.ActivityController
import org.robolectric.shadows.ShadowLooper

@RunWith(RobolectricTestRunner::class)
@Config(sdk = [34])
class MainActivityInsetsTest {
    private val activities = mutableListOf<ActivityController<Activity>>()

    @After
    fun tearDown() {
        activities.forEach { it.close() }
    }

    @Test
    fun systemBarsShrinkTheExistingWebViewViewportWithoutChangingItsPadding() {
        val fixture = fixture()
        fixture.dispatch(bars = Insets.of(0, 72, 0, 120))
        fixture.layout(1080, 2400)

        fixture.assertBounds(8, 84, 1064, 2260)
        assertSame(fixture.container, fixture.webView.parent)
        assertSame(fixture.layoutParams, fixture.webView.layoutParams)
        assertEquals(3, fixture.webView.paddingLeft)
        assertEquals(5, fixture.webView.paddingTop)
        assertEquals(7, fixture.webView.paddingRight)
        assertEquals(9, fixture.webView.paddingBottom)
    }

    @Test
    fun redispatchAndLandscapeCutoutReplaceInsetsRatherThanAccumulateThem() {
        val fixture = fixture()
        repeat(2) { fixture.dispatch(bars = Insets.of(0, 72, 0, 120)) }
        fixture.layout(1080, 2400)
        fixture.assertBounds(8, 84, 1064, 2260)

        fixture.dispatch(
            bars = Insets.of(0, 0, 96, 0),
            cutout = Insets.of(120, 0, 0, 0),
        )
        fixture.layout(2400, 1080)
        fixture.assertBounds(128, 12, 2288, 1060)

        fixture.dispatch()
        fixture.layout(2400, 1080)
        fixture.assertBounds(8, 12, 2384, 1060)
    }

    @Test
    fun handledInsetsAreRemovedDownstreamWhileImeInsetsRemainAvailable() {
        val fixture = fixture()
        var delivered: WindowInsetsCompat? = null
        ViewCompat.setOnApplyWindowInsetsListener(fixture.webView) { _, insets ->
            delivered = insets
            insets
        }
        fixture.dispatch(
            bars = Insets.of(0, 72, 0, 120),
            cutout = Insets.of(20, 80, 0, 0),
            ime = Insets.of(0, 0, 0, 900),
        )
        fixture.layout(1080, 2400)

        fixture.assertBounds(28, 92, 1064, 1480)
        val childInsets = requireNotNull(delivered)
        assertEquals(Insets.NONE, childInsets.getInsets(safeTypes))
        assertEquals(Insets.of(0, 0, 0, 900), childInsets.getInsets(WindowInsetsCompat.Type.ime()))
    }

    @Test
    fun imeShowAndHideResizeViewportWithoutAddingNavigationBarTwice() {
        val fixture = fixture()
        repeat(2) {
            fixture.dispatch(bars = Insets.of(0, 72, 0, 120), ime = Insets.of(0, 0, 0, 900))
        }
        fixture.layout(1080, 2400)
        fixture.assertBounds(8, 84, 1064, 1480)

        fixture.dispatch(bars = Insets.of(0, 72, 0, 120))
        fixture.layout(1080, 2400)
        fixture.assertBounds(8, 84, 1064, 2260)
    }

    @Test
    fun detachAndReattachPreserveTheOriginalPaddingBaseline() {
        val fixture = fixture()
        fixture.dispatch(bars = Insets.of(0, 72, 0, 120))
        fixture.container.removeView(fixture.webView)
        fixture.container.addView(fixture.webView, fixture.layoutParams)
        fixture.dispatch(bars = Insets.of(0, 72, 0, 120))
        fixture.layout(1080, 2400)
        fixture.assertBounds(8, 84, 1064, 2260)
    }

    @Test
    fun hookAlsoWorksWhenWebViewIsAlreadyAttached() {
        val fixture = fixture(installBeforeAttachment = false)
        fixture.dispatch(bars = Insets.of(20, 72, 30, 120))
        fixture.layout(1080, 2400)
        fixture.assertBounds(28, 84, 1034, 2260)
    }

    private fun fixture(installBeforeAttachment: Boolean = true): Fixture {
        val controller = Robolectric.buildActivity(Activity::class.java).setup()
        activities.add(controller)
        val activity = controller.get()
        val container = activity.findViewById<ViewGroup>(android.R.id.content)
        container.setPadding(8, 12, 16, 20)
        val webView = WebView(activity)
        webView.setPadding(3, 5, 7, 9)
        val mainActivity = MainActivity()
        if (installBeforeAttachment) mainActivity.onWebViewCreate(webView)
        activity.setContentView(webView)
        ShadowLooper.idleMainLooper()
        assertTrue("WebView must be attached before dispatch", webView.isAttachedToWindow)
        if (!installBeforeAttachment) mainActivity.onWebViewCreate(webView)
        // Robolectric does not implement Chromium's WebView.setFrame provider.
        // A sibling with the same layout contract verifies Android parent geometry;
        // measured WebView dimensions independently verify the viewport shrinks.
        val geometryProbe = View(activity)
        container.addView(geometryProbe, ViewGroup.LayoutParams(-1, -1))
        return Fixture(container, webView, webView.layoutParams, geometryProbe)
    }

    private class Fixture(
        val container: ViewGroup,
        val webView: WebView,
        val layoutParams: ViewGroup.LayoutParams,
        val geometryProbe: View,
    ) {
        fun dispatch(
            bars: Insets = Insets.NONE,
            cutout: Insets = Insets.NONE,
            ime: Insets = Insets.NONE,
        ) {
            val insets = WindowInsetsCompat.Builder()
                .setInsets(WindowInsetsCompat.Type.systemBars(), bars)
                .setInsets(WindowInsetsCompat.Type.displayCutout(), cutout)
                .setInsets(WindowInsetsCompat.Type.ime(), ime)
                .build()
            ViewCompat.dispatchApplyWindowInsets(container, insets)
        }

        fun layout(width: Int, height: Int) {
            container.measure(
                View.MeasureSpec.makeMeasureSpec(width, View.MeasureSpec.EXACTLY),
                View.MeasureSpec.makeMeasureSpec(height, View.MeasureSpec.EXACTLY),
            )
            container.layout(0, 0, width, height)
        }

        fun assertBounds(left: Int, top: Int, right: Int, bottom: Int) {
            assertEquals("viewport left", left, geometryProbe.left)
            assertEquals("viewport top", top, geometryProbe.top)
            assertEquals("viewport right", right, geometryProbe.right)
            assertEquals("viewport bottom", bottom, geometryProbe.bottom)
            assertEquals("measured viewport width", right - left, webView.measuredWidth)
            assertEquals("measured viewport height", bottom - top, webView.measuredHeight)
        }
    }

    companion object {
        private val safeTypes = WindowInsetsCompat.Type.systemBars() or
            WindowInsetsCompat.Type.displayCutout()
    }
}
