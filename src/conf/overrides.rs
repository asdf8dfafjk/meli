/*
 * meli - conf/overrides.rs
 *
 * Copyright 2020 Manos Pitsidianakis
 *
 * This file is part of meli.
 *
 * meli is free software: you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation, either version 3 of the License, or
 * (at your option) any later version.
 *
 * meli is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with meli. If not, see <http://www.gnu.org/licenses/>.
 */

//! This module is automatically generated by build.rs.
use super::*;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PagerSettingsOverride {
    #[doc = " Number of context lines when going to next page."]
    #[doc = " Default: 0"]
    #[serde(alias = "pager-context")]
    #[serde(default)]
    pub pager_context: Option<usize>,
    #[doc = " Stop at the end instead of displaying next mail."]
    #[doc = " Default: false"]
    #[serde(alias = "pager-stop")]
    #[serde(default)]
    pub pager_stop: Option<bool>,
    #[doc = " Always show headers when scrolling."]
    #[doc = " Default: true"]
    #[serde(alias = "headers-sticky")]
    #[serde(default)]
    pub headers_sticky: Option<bool>,
    #[doc = " The height of the pager in mail view, in percent."]
    #[doc = " Default: 80"]
    #[serde(alias = "pager-ratio")]
    #[serde(default)]
    pub pager_ratio: Option<usize>,
    #[doc = " A command to pipe mail output through for viewing in pager."]
    #[doc = " Default: None"]
    #[serde(deserialize_with = "non_empty_string")]
    #[serde(default)]
    pub filter: Option<Option<String>>,
    #[doc = " A command to pipe html output before displaying it in a pager"]
    #[doc = " Default: None"]
    #[serde(deserialize_with = "non_empty_string", alias = "html-filter")]
    #[serde(default)]
    pub html_filter: Option<Option<String>>,
    #[doc = " Respect \"format=flowed\""]
    #[doc = " Default: true"]
    #[serde(alias = "format-flowed")]
    #[serde(default)]
    pub format_flowed: Option<bool>,
    #[doc = " Split long lines that would overflow on the x axis."]
    #[doc = " Default: true"]
    #[serde(alias = "split-long-lines")]
    #[serde(default)]
    pub split_long_lines: Option<bool>,
    #[doc = " Minimum text width in columns."]
    #[doc = " Default: 80"]
    #[serde(alias = "minimum-width")]
    #[serde(default)]
    pub minimum_width: Option<usize>,
    #[doc = " Choose `text/html` alternative if `text/plain` is empty in `multipart/alternative`"]
    #[doc = " attachments."]
    #[doc = " Default: true"]
    #[serde(alias = "auto-choose-multipart-alternative")]
    #[serde(default)]
    pub auto_choose_multipart_alternative: Option<ToggleFlag>,
}
impl Default for PagerSettingsOverride {
    fn default() -> Self {
        PagerSettingsOverride {
            pager_context: None,
            pager_stop: None,
            headers_sticky: None,
            pager_ratio: None,
            filter: None,
            html_filter: None,
            format_flowed: None,
            split_long_lines: None,
            minimum_width: None,
            auto_choose_multipart_alternative: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ListingSettingsOverride {
    #[doc = " Number of context lines when going to next page."]
    #[doc = " Default: 0"]
    #[serde(alias = "context-lines")]
    #[serde(default)]
    pub context_lines: Option<usize>,
    #[doc = " Datetime formatting passed verbatim to strftime(3)."]
    #[doc = " Default: %Y-%m-%d %T"]
    #[serde(alias = "datetime-fmt")]
    #[serde(default)]
    pub datetime_fmt: Option<Option<String>>,
    #[doc = " Show recent dates as `X {minutes,hours,days} ago`, up to 7 days."]
    #[doc = " Default: true"]
    #[serde(alias = "recent-dates")]
    #[serde(default)]
    pub recent_dates: Option<bool>,
    #[doc = " Show only envelopes that match this query"]
    #[doc = " Default: None"]
    #[serde(default)]
    pub filter: Option<Option<Query>>,
    #[serde(alias = "index-style")]
    #[serde(default)]
    pub index_style: Option<IndexStyle>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_has_sibling: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_no_sibling: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_has_sibling_leaf: Option<Option<String>>,
    #[doc = "Default: \" \""]
    #[serde(default)]
    pub sidebar_mailbox_tree_no_sibling_leaf: Option<Option<String>>,
}
impl Default for ListingSettingsOverride {
    fn default() -> Self {
        ListingSettingsOverride {
            context_lines: None,
            datetime_fmt: None,
            recent_dates: None,
            filter: None,
            index_style: None,
            sidebar_mailbox_tree_has_sibling: None,
            sidebar_mailbox_tree_no_sibling: None,
            sidebar_mailbox_tree_has_sibling_leaf: None,
            sidebar_mailbox_tree_no_sibling_leaf: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct NotificationsSettingsOverride {
    #[doc = " Enable notifications."]
    #[doc = " Default: True"]
    #[serde(default)]
    pub enable: Option<bool>,
    #[doc = " A command to pipe notifications through"]
    #[doc = " Default: None"]
    #[serde(default)]
    pub script: Option<Option<String>>,
    #[doc = " A file location which has its size changed when new mail arrives (max 128 bytes). Can be"]
    #[doc = " used to trigger new mail notifications eg with `xbiff(1)`"]
    #[doc = " Default: None"]
    #[serde(alias = "xbiff-file-path")]
    #[serde(default)]
    pub xbiff_file_path: Option<Option<String>>,
    #[serde(alias = "play-sound")]
    #[serde(default)]
    pub play_sound: Option<ToggleFlag>,
    #[serde(alias = "sound-file")]
    #[serde(default)]
    pub sound_file: Option<Option<String>>,
}
impl Default for NotificationsSettingsOverride {
    fn default() -> Self {
        NotificationsSettingsOverride {
            enable: None,
            script: None,
            xbiff_file_path: None,
            play_sound: None,
            sound_file: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ShortcutsOverride {
    #[serde(default)]
    pub general: Option<GeneralShortcuts>,
    #[serde(default)]
    pub listing: Option<ListingShortcuts>,
    #[serde(default)]
    pub composing: Option<ComposingShortcuts>,
    #[serde(alias = "compact-listing")]
    #[serde(default)]
    pub compact_listing: Option<CompactListingShortcuts>,
    #[serde(alias = "contact-list")]
    #[serde(default)]
    pub contact_list: Option<ContactListShortcuts>,
    #[serde(alias = "envelope-view")]
    #[serde(default)]
    pub envelope_view: Option<EnvelopeViewShortcuts>,
    #[serde(alias = "thread-view")]
    #[serde(default)]
    pub thread_view: Option<ThreadViewShortcuts>,
    #[serde(default)]
    pub pager: Option<PagerShortcuts>,
}
impl Default for ShortcutsOverride {
    fn default() -> Self {
        ShortcutsOverride {
            general: None,
            listing: None,
            composing: None,
            compact_listing: None,
            contact_list: None,
            envelope_view: None,
            thread_view: None,
            pager: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct ComposingSettingsOverride {
    #[doc = " A command to pipe new emails to"]
    #[doc = " Required"]
    #[serde(default)]
    pub send_mail: Option<SendMail>,
    #[doc = " Command to launch editor. Can have arguments. Draft filename is given as the last argument. If it's missing, the environment variable $EDITOR is looked up."]
    #[serde(alias = "editor-command", alias = "editor-cmd", alias = "editor_cmd")]
    #[serde(default)]
    pub editor_command: Option<Option<String>>,
    #[doc = " Embed editor (for terminal interfaces) instead of forking and waiting."]
    #[serde(default)]
    pub embed: Option<bool>,
    #[doc = " Set \"format=flowed\" in plain text attachments."]
    #[doc = " Default: true"]
    #[serde(alias = "format-flowed")]
    #[serde(default)]
    pub format_flowed: Option<bool>,
    #[doc = "Set User-Agent"]
    #[doc = "Default: empty"]
    #[serde(alias = "insert_user_agent")]
    #[serde(default)]
    pub insert_user_agent: Option<bool>,
    #[doc = " Set default header values for new drafts"]
    #[doc = " Default: empty"]
    #[serde(alias = "default-header-values")]
    #[serde(default)]
    pub default_header_values: Option<HashMap<String, String>>,
}
impl Default for ComposingSettingsOverride {
    fn default() -> Self {
        ComposingSettingsOverride {
            send_mail: None,
            editor_command: None,
            embed: None,
            format_flowed: None,
            insert_user_agent: None,
            default_header_values: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct TagsSettingsOverride {
    #[serde(deserialize_with = "tag_color_de")]
    #[serde(default)]
    pub colors: Option<HashMap<u64, Color>>,
    #[serde(deserialize_with = "tag_set_de", alias = "ignore-tags")]
    #[serde(default)]
    pub ignore_tags: Option<HashSet<u64>>,
}
impl Default for TagsSettingsOverride {
    fn default() -> Self {
        TagsSettingsOverride {
            colors: None,
            ignore_tags: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct PGPSettingsOverride {
    #[doc = " auto verify signed e-mail according to RFC3156"]
    #[serde(alias = "auto-verify-signatures")]
    #[serde(default)]
    pub auto_verify_signatures: Option<bool>,
    #[doc = " always sign sent messages"]
    #[serde(alias = "auto-sign")]
    #[serde(default)]
    pub auto_sign: Option<bool>,
    #[serde(default)]
    pub key: Option<Option<String>>,
    #[doc = " gpg binary name or file location to use"]
    #[serde(alias = "gpg-binary")]
    #[serde(default)]
    pub gpg_binary: Option<Option<String>>,
}
impl Default for PGPSettingsOverride {
    fn default() -> Self {
        PGPSettingsOverride {
            auto_verify_signatures: None,
            auto_sign: None,
            key: None,
            gpg_binary: None,
        }
    }
}
