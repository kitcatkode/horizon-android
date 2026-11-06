package dev.horizon.android;

/**
 * SAF result callback, the second screen's host and the wallpaper's colours. The native
 * activity, UI and file operations stay Rust.
 */
public final class HorizonActivity extends android.app.NativeActivity {
    static { System.loadLibrary("horizon_android"); }
    private native void nativeDocumentResult(int request, int result, String uri);
    private SecondScreen secondScreen;

    @Override protected void onCreate(android.os.Bundle state) {
        super.onCreate(state);
        secondScreen = new SecondScreen(this);
    }

    @Override protected void onStart() {
        super.onStart();
        secondScreen.start();
    }

    @Override protected void onStop() {
        secondScreen.stop();
        super.onStop();
    }

    /** Material You's dark scheme for the Material 3 interface (MaterialColors). */
    public int[] materialScheme() {
        return MaterialColors.scheme(this);
    }

    @Override protected void onActivityResult(int request, int result, android.content.Intent data) {
        super.onActivityResult(request, result, data);
        if (request != 4101) return;
        nativeDocumentResult(request, result,
                data == null || data.getData() == null ? null : data.getData().toString());
    }
}
