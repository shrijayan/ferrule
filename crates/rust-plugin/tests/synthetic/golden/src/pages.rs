//! The API's `pagination` schemes.

use crate::rt::{PageConfig, ParamLocation, Scheme};

/// The `items_cursor` scheme.
pub const ITEMS_CURSOR: PageConfig = PageConfig {
    scheme: Scheme::Cursor,
    location: ParamLocation::Query,
    params: &[("next_cursor_param", "after")],
    fields: &[("items", "data"), ("next_cursor_field", "next_cursor")],
};

/// The `offset_page` scheme.
pub const OFFSET_PAGE: PageConfig = PageConfig {
    scheme: Scheme::Offset,
    location: ParamLocation::Query,
    params: &[("offset_count_param", "offset")],
    fields: &[("items", "results"), ("offset_total_count_field", "total")],
};

/// The `numbered_page` scheme.
pub const NUMBERED_PAGE: PageConfig = PageConfig {
    scheme: Scheme::PageNumber,
    location: ParamLocation::Query,
    params: &[("page_number_param", "page")],
    fields: &[
        ("items", "entries"),
        ("current_page_number_field", "page"),
        ("total_page_count_field", "total_pages"),
    ],
};

/// The `id_cursor` scheme.
pub const ID_CURSOR: PageConfig = PageConfig {
    scheme: Scheme::CursorId,
    location: ParamLocation::Query,
    params: &[
        ("next_cursor_id_param", "starting_after"),
        ("previous_cursor_id_param", "ending_before"),
    ],
    fields: &[("cursor_item_id", "objects.[].id"), ("items", "objects")],
};

/// The `url_cursor` scheme.
pub const URL_CURSOR: PageConfig = PageConfig {
    scheme: Scheme::CursorUrl,
    location: ParamLocation::Query,
    params: &[],
    fields: &[("items", "logs"), ("cursor_url_field", "next")],
};

/// The `single_page` scheme.
pub const SINGLE_PAGE: PageConfig = PageConfig {
    scheme: Scheme::FakePage,
    location: ParamLocation::Query,
    params: &[],
    fields: &[("items", "labels")],
};
