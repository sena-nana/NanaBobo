use nana_ui::runtime::{AppContext, FrameworkError, SemanticColorRole as Role};
use nana_ui::theme::{ThemeDefinition, ThemeId};
use nana_ui::{AppearanceSettings, ButtonKind, ThemeMode, ThemeModeExt};

pub(super) fn install(
    cx: &mut AppContext,
    mode: ThemeMode,
    appearance: AppearanceSettings,
) -> Result<(), FrameworkError> {
    cx.set_theme_definition(&definition(mode, appearance))?;
    Ok(())
}

fn definition(mode: ThemeMode, appearance: AppearanceSettings) -> ThemeDefinition {
    let id = match mode {
        ThemeMode::Light => "nanabobo.halo.light",
        ThemeMode::Dark => "nanabobo.halo.dark",
    };
    let mut metrics = appearance.metrics();
    let md = appearance.radius_md();
    metrics.radius_sm = (md * 0.56).round();
    metrics.radius_lg = (md * 1.22).round();
    metrics.radius_xl = (md * 1.58).round();
    metrics.control_padding_x = 13.0;
    metrics.icon_button_size = 30.0;
    let mut theme = mode
        .definition()
        .with_id(ThemeId::new(id))
        .with_palette(super::palette::palette(mode))
        .with_metrics(metrics);
    theme.typography.section = 15.0;
    theme.typography.heading = 18.0;
    theme.typography.title = 21.0;
    theme.effects.surface.offset_y = 6.0;
    theme.effects.surface.blur_radius = 22.0;
    theme.effects.surface.spread_radius = -8.0;
    for (kind, foreground, background, hover, pressed) in [
        (
            ButtonKind::Primary,
            Role::AccentText,
            Role::Accent,
            Role::AccentStrong,
            Role::AccentOnSoft,
        ),
        (
            ButtonKind::Subtle,
            Role::AccentOnSoft,
            Role::Selected,
            Role::SelectedHover,
            Role::SelectedPressed,
        ),
    ] {
        let mut variant = theme.components.button.variant(kind);
        variant.foreground = Some(foreground);
        variant.background = Some(Some(background));
        variant.border = Some(None);
        variant.hovered_background = Some(hover);
        variant.pressed_background = Some(pressed);
        theme.components.button = theme.components.button.with(kind, variant);
    }
    theme.bump()
}
