package dev.horizon.android;

import android.app.Activity;
import android.app.ActivityOptions;
import android.content.Intent;
import android.os.Bundle;
import android.view.Display;

/**
 * The launcher entry. A dual-screen device's launcher on the lower screen (the AYN Thor) starts
 * apps there, but the game belongs on the main display, with the lower screen for its HUD panel
 * (SecondScreen). This starts the game on the default display and closes; it never loads the
 * native library, so the game's single native instance is not involved.
 */
public final class HorizonLauncher extends Activity {
    @Override protected void onCreate(Bundle state) {
        super.onCreate(state);
        Intent launch = new Intent(getIntent());
        launch.setClass(this, HorizonActivity.class);
        launch.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK);
        ActivityOptions options = ActivityOptions.makeBasic();
        options.setLaunchDisplayId(Display.DEFAULT_DISPLAY);
        try {
            startActivity(launch, options.toBundle());
        } catch (RuntimeException e) {
            // A display the system will not launch on: wherever the launcher is.
            startActivity(launch);
        }
        finish();
        overridePendingTransition(0, 0);
    }
}
