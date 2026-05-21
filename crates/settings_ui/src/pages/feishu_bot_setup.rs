use gpui::{Action as _, ReadGlobal, ScrollHandle, prelude::*};
use settings::SettingsStore;
use theme::translate;
use ui::{
    Button, ButtonSize, ButtonStyle, Color, Divider, Icon, IconName, IconSize, Label, LabelSize,
    prelude::*,
};

use crate::SettingsWindow;

/// Render the Feishu Bot binding setup sub-page.
pub(crate) fn render_feishu_bot_setup_page(
    _settings_window: &SettingsWindow,
    scroll_handle: &ScrollHandle,
    _window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let settings = SettingsStore::global(cx).merged_settings();
    let feishu_bot = settings.feishu_bot.as_ref();
    let enabled = feishu_bot.and_then(|f| f.enabled).unwrap_or(false);
    let bound = feishu_bot.and_then(|f| f.bound).unwrap_or(false);
    let bound_user_id = feishu_bot
        .and_then(|f| f.bound_user_id.as_deref())
        .unwrap_or("");
    let bound_workspace = feishu_bot
        .and_then(|f| f.bound_workspace.as_deref())
        .unwrap_or("");

    v_flex()
        .id("feishu-bot-setup-page")
        .min_w_0()
        .size_full()
        .pt_2p5()
        .px_8()
        .pb_16()
        .gap_4()
        .overflow_y_scroll()
        .track_scroll(scroll_handle)
        .child(render_status_section(
            enabled,
            bound,
            bound_user_id,
            bound_workspace,
            cx,
        ))
        .child(Divider::horizontal())
        .child(render_bind_section(cx))
        .when(bound, |this| {
            this.child(Divider::horizontal())
                .child(render_unbind_section(cx))
        })
        .into_any_element()
}

fn render_status_section(
    enabled: bool,
    bound: bool,
    bound_user_id: &str,
    bound_workspace: &str,
    cx: &App,
) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(Label::new(translate("Bot Status", cx)).size(LabelSize::Large))
        .child(
            h_flex()
                .gap_2()
                .child(Label::new(translate("Enabled:", cx)).color(Color::Muted))
                .child(
                    Label::new(translate(if enabled { "Yes" } else { "No" }, cx)).color(
                        if enabled {
                            Color::Created
                        } else {
                            Color::Muted
                        },
                    ),
                ),
        )
        .child(
            h_flex()
                .gap_2()
                .child(Label::new(translate("Bound:", cx)).color(Color::Muted))
                .child({
                    let (text, color) = if bound {
                        (translate("Connected", cx), Color::Created)
                    } else {
                        (translate("Not Connected", cx), Color::Muted)
                    };
                    Label::new(text).color(color)
                }),
        )
        .when(bound && !bound_user_id.is_empty(), |this| {
            this.child(
                h_flex()
                    .gap_2()
                    .child(Label::new(translate("User:", cx)).color(Color::Muted))
                    .child(Label::new(bound_user_id.to_string())),
            )
        })
        .when(bound && !bound_workspace.is_empty(), |this| {
            this.child(
                h_flex()
                    .gap_2()
                    .child(Label::new(translate("Workspace:", cx)).color(Color::Muted))
                    .child(Label::new(bound_workspace.to_string())),
            )
        })
}

fn render_bind_section(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(Label::new(translate("Bind Bot", cx)).size(LabelSize::Large))
        .child(
            Label::new(translate(
                "Generate a token from the command palette (\"Generate Bot Token\"), then paste it below and click Bind.",
                cx,
            ))
            .size(LabelSize::Small)
            .color(Color::Muted),
        )
        .child(
            h_flex()
                .gap_2()
                .items_center()
                .child(
                    Button::new("bind-bot", translate("Bind", cx))
                        .style(ButtonStyle::Filled)
                        .size(ButtonSize::Medium)
                        .on_click(cx.listener(|_this, _, window, cx| {
                            window.dispatch_action(
                                zed_actions::FeishuBotBind.boxed_clone(),
                                cx,
                            );
                        })),
                )
                .child(
                    Button::new("generate-token", translate("Generate Token", cx))
                        .style(ButtonStyle::OutlinedGhost)
                        .size(ButtonSize::Medium)
                        .end_icon(
                            Icon::new(IconName::ArrowUpRight)
                                .size(IconSize::Small)
                                .color(Color::Muted),
                        )
                        .on_click(cx.listener(|_this, _, window, cx| {
                            window.dispatch_action(
                                zed_actions::FeishuBotGenerateToken.boxed_clone(),
                                cx,
                            );
                        })),
                ),
        )
}

fn render_unbind_section(cx: &mut Context<SettingsWindow>) -> impl IntoElement {
    v_flex()
        .gap_2()
        .child(Label::new(translate("Unbind Bot", cx)).size(LabelSize::Large))
        .child(
            Label::new(translate(
                "Disconnect the bot from the current workspace. You can re-bind at any time with a new token.",
                cx,
            ))
            .size(LabelSize::Small)
            .color(Color::Muted),
        )
        .child(
            Button::new("unbind-bot", translate("Unbind", cx))
                .style(ButtonStyle::Outlined)
                .size(ButtonSize::Medium)
                .color(Color::Warning)
                .on_click(cx.listener(|_this, _, window, cx| {
                    window.dispatch_action(
                        zed_actions::FeishuBotUnbind.boxed_clone(),
                        cx,
                    );
                })),
        )
}
