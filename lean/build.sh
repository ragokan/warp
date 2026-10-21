#!/usr/bin/env bash
set -euo pipefail
repo=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
cd "$repo"
features=lean,jemalloc,osc_hyperlinks,render_continuous_block_selections_with_single_border,shell_selector,minimalist_ui,avatar_in_tab_bar,full_screen_zen_mode,ligatures,rect_selection,kitty_images,validate_autosuggestions,clear_autosuggestion_on_escape,tab_close_button_on_left,allow_opening_file_links_using_editor_env,undo_closed_panes,ui_zoom,async_find,classic_completions,force_classic_completions,pluggable_notifications,new_tab_styling,richtext_multiselect,kitty_keyboard_protocol,trim_trailing_blank_lines,skip_firebase_anonymous_user,settings_file,terminal_lifecycle_recovery,ime_marked_text,native_shell_completions,history_search_ranking_v2
cargo build --locked --package warp --bin warp-oss --profile lean \
    --no-default-features --features "$features" "$@"
printf '%s/lean/warp-oss\n' "${CARGO_TARGET_DIR:-$repo/target}${CARGO_BUILD_TARGET:+/$CARGO_BUILD_TARGET}"
