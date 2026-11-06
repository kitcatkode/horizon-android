package dev.horizon.android;

import android.app.Activity;
import android.app.Presentation;
import android.content.Context;
import android.hardware.display.DisplayManager;
import android.os.Build;
import android.os.Bundle;
import android.os.Handler;
import android.os.Looper;
import android.view.Display;
import android.view.MotionEvent;
import android.view.SurfaceHolder;
import android.view.SurfaceView;
import android.view.View;
import android.view.Window;
import android.view.WindowInsets;
import android.view.WindowInsetsController;
import android.view.WindowManager;

/**
 * The lower screen of a dual-screen device (the AYN Thor; any presentation display): a
 * Presentation whose SurfaceView the game draws its HUD panel into, from Rust. Its window
 * never takes focus, so the controller's keys and sticks stay with the game's activity.
 */
final class SecondScreen implements DisplayManager.DisplayListener {
    private static final int STARTED = 0, MOVED = 1, ENDED = 2, CANCELLED = 3;

    private final Activity activity;
    private final DisplayManager displays;
    private final Handler handler = new Handler(Looper.getMainLooper());
    private Panel panel;
    private boolean started;

    SecondScreen(Activity activity) {
        this.activity = activity;
        this.displays = (DisplayManager) activity.getSystemService(Context.DISPLAY_SERVICE);
    }

    void start() {
        if (started || displays == null) return;
        started = true;
        displays.registerDisplayListener(this, handler);
        update();
    }

    void stop() {
        if (!started) return;
        started = false;
        displays.unregisterDisplayListener(this);
        dismiss();
    }

    @Override public void onDisplayAdded(int id) { update(); }
    @Override public void onDisplayRemoved(int id) { update(); }
    @Override public void onDisplayChanged(int id) { update(); }

    /** A presentation display other than the game's own (Android refuses a presentation there). */
    @SuppressWarnings("deprecation")
    private Display target() {
        int own = activity.getWindowManager().getDefaultDisplay().getDisplayId();
        for (Display display : displays.getDisplays(DisplayManager.DISPLAY_CATEGORY_PRESENTATION)) {
            int id = display.getDisplayId();
            if (id != own && id != Display.DEFAULT_DISPLAY && display.isValid()) return display;
        }
        return null;
    }

    private void update() {
        Display target = started ? target() : null;
        if (panel != null && (target == null || panel.getDisplay().getDisplayId() != target.getDisplayId())) {
            dismiss();
        }
        if (panel == null && target != null && !activity.isFinishing()) {
            Panel shown = new Panel(activity, target);
            try {
                shown.show();
                panel = shown;
            } catch (WindowManager.InvalidDisplayException e) {
                android.util.Log.w("Horizon", "Second screen unavailable: " + e.getMessage());
            }
        }
    }

    private void dismiss() {
        if (panel != null) {
            panel.dismiss();
            panel = null;
        }
    }

    static native void nativeSurface(android.view.Surface surface, int width, int height);
    static native void nativeTouch(int phase, int id, float x, float y);

    private static final class Panel extends Presentation implements SurfaceHolder.Callback {
        Panel(Context context, Display display) {
            super(context, display);
        }

        @Override protected void onCreate(Bundle state) {
            super.onCreate(state);
            Window window = getWindow();
            window.addFlags(WindowManager.LayoutParams.FLAG_NOT_FOCUSABLE
                    | WindowManager.LayoutParams.FLAG_KEEP_SCREEN_ON);
            SurfaceView view = new SurfaceView(getContext());
            view.getHolder().addCallback(this);
            view.setOnTouchListener(Panel::touch);
            setContentView(view);
            if (Build.VERSION.SDK_INT >= 30) {
                WindowInsetsController insets = window.getInsetsController();
                if (insets != null) {
                    insets.hide(WindowInsets.Type.systemBars());
                    insets.setSystemBarsBehavior(WindowInsetsController.BEHAVIOR_SHOW_TRANSIENT_BARS_BY_SWIPE);
                }
            } else {
                view.setSystemUiVisibility(View.SYSTEM_UI_FLAG_FULLSCREEN
                        | View.SYSTEM_UI_FLAG_HIDE_NAVIGATION
                        | View.SYSTEM_UI_FLAG_IMMERSIVE_STICKY);
            }
        }

        @Override public void surfaceCreated(SurfaceHolder holder) {}

        @Override public void surfaceChanged(SurfaceHolder holder, int format, int width, int height) {
            nativeSurface(holder.getSurface(), width, height);
        }

        @Override public void surfaceDestroyed(SurfaceHolder holder) {
            nativeSurface(null, 0, 0);
        }

        private static boolean touch(View view, MotionEvent event) {
            int action = event.getActionMasked();
            switch (action) {
                case MotionEvent.ACTION_DOWN:
                case MotionEvent.ACTION_POINTER_DOWN:
                case MotionEvent.ACTION_UP:
                case MotionEvent.ACTION_POINTER_UP: {
                    int i = event.getActionIndex();
                    boolean down = action == MotionEvent.ACTION_DOWN || action == MotionEvent.ACTION_POINTER_DOWN;
                    nativeTouch(down ? STARTED : ENDED, event.getPointerId(i), event.getX(i), event.getY(i));
                    break;
                }
                case MotionEvent.ACTION_MOVE:
                case MotionEvent.ACTION_CANCEL: {
                    int phase = action == MotionEvent.ACTION_MOVE ? MOVED : CANCELLED;
                    for (int i = 0; i < event.getPointerCount(); i++) {
                        nativeTouch(phase, event.getPointerId(i), event.getX(i), event.getY(i));
                    }
                    break;
                }
                default:
                    return false;
            }
            return true;
        }
    }
}
