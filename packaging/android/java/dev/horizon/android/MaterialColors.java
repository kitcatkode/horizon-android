package dev.horizon.android;

import android.content.Context;
import android.content.res.Resources;
import android.os.Build;

/**
 * Material You's dark colour scheme, from the wallpaper, for the launcher's and the game's
 * Material 3 interface (crates/horizon-client/src/ui_material.rs, in its ROLES order). Android
 * 14 has every role as a system colour; Android 12 and 13 only have the tonal palettes, so the
 * roles come from the tones Material 3's dark scheme uses. A role left 0 keeps the built-in
 * baseline scheme's.
 */
final class MaterialColors {
    private static final String[] ROLES = {
        "primary", "on_primary", "primary_container", "on_primary_container",
        "secondary", "on_secondary", "secondary_container", "on_secondary_container",
        "tertiary", "on_tertiary", "tertiary_container", "on_tertiary_container",
        "error", "on_error", "error_container", "on_error_container",
        "surface", "on_surface", "on_surface_variant",
        "surface_container_lowest", "surface_container_low", "surface_container",
        "surface_container_high", "surface_container_highest",
        "outline", "outline_variant",
        "inverse_surface", "inverse_on_surface", "inverse_primary",
    };

    /** The tonal palette colour each role takes before Android 14 (palette_tone). */
    private static final String[] TONES = {
        "accent1_200", "accent1_800", "accent1_700", "accent1_100",
        "accent2_200", "accent2_800", "accent2_700", "accent2_100",
        "accent3_200", "accent3_800", "accent3_700", "accent3_100",
        null, null, null, null,
        "neutral1_900", "neutral1_100", "neutral2_200",
        "neutral1_1000", "neutral1_900", "neutral1_900",
        "neutral1_800", "neutral1_800",
        "neutral2_400", "neutral2_700",
        "neutral1_100", "neutral1_800", "accent1_600",
    };

    private MaterialColors() {}

    static int[] scheme(Context context) {
        int[] out = new int[ROLES.length];
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.S) {
            return out;
        }
        Resources resources = context.getResources();
        boolean roles = Build.VERSION.SDK_INT >= Build.VERSION_CODES.UPSIDE_DOWN_CAKE;
        for (int i = 0; i < ROLES.length; i++) {
            String name = roles ? "system_" + ROLES[i] + "_dark"
                    : TONES[i] == null ? null : "system_" + TONES[i];
            out[i] = color(context, resources, name);
        }
        return out;
    }

    private static int color(Context context, Resources resources, String name) {
        if (name == null) {
            return 0;
        }
        int id = resources.getIdentifier(name, "color", "android");
        if (id == 0) {
            return 0;
        }
        try {
            return resources.getColor(id, context.getTheme());
        } catch (Resources.NotFoundException e) {
            return 0;
        }
    }
}
