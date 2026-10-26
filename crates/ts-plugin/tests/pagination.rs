//! Pagination in both layouts, from a small fixture (`tests/fixtures/pages`): a `cursor` scheme
//! (knock-mgmt-typescript's `entries_cursor`) and a `page_number` scheme (honcho-typescript's
//! `page`), a GET list and a POST list, a list whose items are declared in another resource's
//! module, and one whose items are an inline object.
//!
//! `src/core/pagination.ts` (current layout) and `src/pagination.ts` (`node` layout) are compared
//! whole against `tests/fixtures/pages/expected/`: the oracles' text (knock-mgmt-node `ca88fdba`,
//! honcho-node-core `91d25806`) with the fixture's names. The rest is checked line by line.

mod support;

use std::path::Path;

use ferrule_ts_plugin::{Layout, generate_layout};

fn files(layout: Layout) -> Vec<(String, String)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages");
    let read = |p: &str| std::fs::read_to_string(dir.join(p)).expect("fixture");
    let (client, meta, extras) =
        support::load(&read("openapi.yml"), &read("stainless.yml")).expect("the fixture maps to the model");
    generate_layout(&client, &meta, &extras, layout)
}

fn file<'a>(files: &'a [(String, String)], path: &str) -> &'a str {
    files
        .iter()
        .find(|(p, _)| p == path)
        .map_or_else(|| panic!("{path} is not generated"), |(_, t)| t.as_str())
}

fn expected(name: &str) -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/pages/expected");
    std::fs::read_to_string(dir.join(name)).expect("expected file")
}

#[track_caller]
fn has(text: &str, snippet: &str) {
    assert!(text.contains(snippet), "missing:\n{snippet}\n--- in:\n{text}");
}

#[test]
fn writes_the_page_classes() {
    let current = files(Layout::Typescript);
    assert_eq!(
        file(&current, "src/core/pagination.ts"),
        expected("typescript/pagination.ts")
    );
    assert_eq!(
        file(&current, "src/pagination.ts"),
        "/** @deprecated Import from ./core/pagination instead */\nexport * from './core/pagination';\n"
    );
    let node = files(Layout::Node);
    assert_eq!(file(&node, "src/pagination.ts"), expected("node/pagination.ts"));
    assert!(!node.iter().any(|(p, _)| p == "src/core/pagination.ts"));
}

#[test]
fn list_methods_return_pages() {
    let f = files(Layout::Typescript);
    let widgets = file(&f, "src/resources/widgets.ts");
    has(
        widgets,
        "import { APIResource } from '../core/resource';
import {
  EntriesCursor,
  type EntriesCursorParams,
  Page,
  type PageParams,
  PagePromise,
} from '../core/pagination';
import { RequestOptions } from '../internal/request-options';
",
    );
    has(
        widgets,
        "   * @example
   * ```ts
   * // Automatically fetches more pages as needed.
   * for await (const widget of client.widgets.list({
   *   environment: 'environment',
   * })) {
   *   // ...
   * }
   * ```
   */
  list(query: WidgetListParams, options?: RequestOptions): PagePromise<WidgetsEntriesCursor, Widget> {
    return this._client.getAPIList('/widgets', EntriesCursor<Widget>, { query, ...options });
  }
",
    );
    has(
        widgets,
        "  search(params: WidgetSearchParams, options?: RequestOptions): PagePromise<WidgetsPage, Widget> {
    const { page, size, ...body } = params;
    return this._client.getAPIList('/widgets/search', Page<Widget>, {
      query: { page, size },
      body,
      method: 'post',
      ...options,
    });
  }
}

export type WidgetsEntriesCursor = EntriesCursor<Widget>;

export type WidgetsPage = Page<Widget>;

/**
 * A widget.
 */
export interface Widget {
",
    );
    has(
        widgets,
        "export interface WidgetListParams extends EntriesCursorParams {
  /**
   * The environment slug.
   */
  environment: string;
}
",
    );
    has(widgets, "export interface WidgetSearchParams extends PageParams {\n");
    has(
        widgets,
        "    type Widget as Widget,
    type WidgetsEntriesCursor as WidgetsEntriesCursor,
    type WidgetsPage as WidgetsPage,
    type WidgetListParams as WidgetListParams,
    type WidgetSearchParams as WidgetSearchParams,
",
    );
}

#[test]
fn pages_of_other_modules_are_imported() {
    let f = files(Layout::Typescript);
    let gadgets = file(&f, "src/resources/gadgets.ts");
    has(
        gadgets,
        "import { APIResource } from '../core/resource';
import * as WidgetsAPI from './widgets';
import { WidgetsEntriesCursor } from './widgets';
import { EntriesCursor, type EntriesCursorParams, PagePromise } from '../core/pagination';
import { RequestOptions } from '../internal/request-options';
import { path } from '../internal/utils/path';
",
    );
    has(
        gadgets,
        "   * for await (const gadgetListResponse of client.gadgets.list()) {
   *   // ...
   * }
   * ```
   */
  list(
    query: GadgetListParams | null | undefined = {},
    options?: RequestOptions,
  ): PagePromise<GadgetListResponsesEntriesCursor, GadgetListResponse> {
    return this._client.getAPIList('/gadgets', EntriesCursor<GadgetListResponse>, { query, ...options });
  }
",
    );
    has(
        gadgets,
        "   * for await (const widget of client.gadgets.listWidgets(
   *   'gadget_id',
   * )) {
   *   // ...
   * }
   * ```
   */
  listWidgets(
    gadgetID: string,
    query: GadgetListWidgetsParams | null | undefined = {},
    options?: RequestOptions,
  ): PagePromise<WidgetsEntriesCursor, WidgetsAPI.Widget> {
    return this._client.getAPIList(path`/gadgets/${gadgetID}/widgets`, EntriesCursor<WidgetsAPI.Widget>, {
      query,
      ...options,
    });
  }
}

export type GadgetListResponsesEntriesCursor = EntriesCursor<GadgetListResponse>;

/**
 * A gadget.
 */
export interface GadgetListResponse {
  id: string;
}

export interface GadgetListParams extends EntriesCursorParams {}

export interface GadgetListWidgetsParams extends EntriesCursorParams {}

export declare namespace Gadgets {
  export {
    type GadgetListResponse as GadgetListResponse,
    type GadgetListResponsesEntriesCursor as GadgetListResponsesEntriesCursor,
    type GadgetListParams as GadgetListParams,
    type GadgetListWidgetsParams as GadgetListWidgetsParams,
  };
}

export { type WidgetsEntriesCursor };
",
    );
}

#[test]
fn exports_the_pages() {
    let f = files(Layout::Typescript);
    has(
        file(&f, "src/resources/index.ts"),
        "export {
  Gadgets,
  type GadgetListResponse,
  type GadgetListParams,
  type GadgetListWidgetsParams,
  type GadgetListResponsesEntriesCursor,
} from './gadgets';
export {
  Widgets,
  type Widget,
  type WidgetListParams,
  type WidgetSearchParams,
  type WidgetsEntriesCursor,
  type WidgetsPage,
} from './widgets';
",
    );
    let client = file(&f, "src/client.ts");
    has(
        client,
        "import * as Errors from './core/error';
import * as Pagination from './core/pagination';
import {
  AbstractPage,
  type EntriesCursorParams,
  EntriesCursorResponse,
  type PageParams,
  PageResponse,
} from './core/pagination';
import * as Uploads from './core/uploads';
",
    );
    has(
        client,
        "  getAPIList<Item, PageClass extends Pagination.AbstractPage<Item>",
    );
    has(
        client,
        "  export type RequestOptions = Opts.RequestOptions;

  export import EntriesCursor = Pagination.EntriesCursor;
  export {
    type EntriesCursorParams as EntriesCursorParams,
    type EntriesCursorResponse as EntriesCursorResponse,
  };

  export import Page = Pagination.Page;
  export { type PageParams as PageParams, type PageResponse as PageResponse };

  export {
    Widgets as Widgets,
",
    );
    let api = file(&f, "api.md");
    has(
        api,
        "- <code title=\"get /widgets\">client.widgets.<a href=\"./src/resources/widgets.ts\">list</a>({ ...params }) -> WidgetsEntriesCursor</code>
- <code title=\"post /widgets/search\">client.widgets.<a href=\"./src/resources/widgets.ts\">search</a>({ ...params }) -> WidgetsPage</code>
",
    );
    assert!(
        !api.contains(">WidgetsPage</a>"),
        "page aliases are not listed as types"
    );
}

#[test]
fn node_list_methods_return_page_classes() {
    let f = files(Layout::Node);
    let widgets = file(&f, "src/resources/widgets.ts");
    has(
        widgets,
        "import { APIResource } from '../resource';
import * as Core from '../core';
import { EntriesCursor, type EntriesCursorParams, Page, type PageParams } from '../pagination';
",
    );
    has(
        widgets,
        "    return this._client.getAPIList('/widgets', WidgetsEntriesCursor, { query, ...options });
  }
",
    );
    has(
        widgets,
        "  search(params: WidgetSearchParams, options?: Core.RequestOptions): Core.PagePromise<WidgetsPage, Widget> {
    const { page, size, ...body } = params;
    return this._client.getAPIList('/widgets/search', WidgetsPage, {
      query: { page, size },
      body,
      method: 'post',
      ...options,
    });
  }
}

export class WidgetsEntriesCursor extends EntriesCursor<Widget> {}

export class WidgetsPage extends Page<Widget> {}
",
    );
    has(
        widgets,
        "Widgets.WidgetsEntriesCursor = WidgetsEntriesCursor;
Widgets.WidgetsPage = WidgetsPage;

export declare namespace Widgets {
  export {
    type Widget as Widget,
    WidgetsEntriesCursor as WidgetsEntriesCursor,
    WidgetsPage as WidgetsPage,
",
    );
    let gadgets = file(&f, "src/resources/gadgets.ts");
    has(
        gadgets,
        "import * as WidgetsAPI from './widgets';
import { WidgetsEntriesCursor } from './widgets';
import { EntriesCursor, type EntriesCursorParams } from '../pagination';
",
    );
    has(
        gadgets,
        "  ): Core.PagePromise<WidgetsEntriesCursor, WidgetsAPI.Widget> {
    if (isRequestOptions(query)) {
      return this.listWidgets(gadgetId, {}, query);
    }
    return this._client.getAPIList(`/gadgets/${gadgetId}/widgets`, WidgetsEntriesCursor, {
      query,
      ...options,
    });
  }
",
    );
    has(gadgets, "\nexport { WidgetsEntriesCursor };\n");
    has(
        file(&f, "src/resources/index.ts"),
        "export {
  WidgetsEntriesCursor,
  WidgetsPage,
  Widgets,
  type Widget,
  type WidgetListParams,
  type WidgetSearchParams,
} from './widgets';
",
    );
    let index = file(&f, "src/index.ts");
    has(
        index,
        "import * as Errors from './error';
import * as Pagination from './pagination';
import { type EntriesCursorParams, EntriesCursorResponse, type PageParams, PageResponse } from './pagination';
import * as Uploads from './uploads';
",
    );
    has(
        index,
        "Acme.Widgets = Widgets;
Acme.WidgetsEntriesCursor = WidgetsEntriesCursor;
Acme.WidgetsPage = WidgetsPage;
",
    );
    has(
        index,
        "  export type RequestOptions = Core.RequestOptions;

  export import EntriesCursor = Pagination.EntriesCursor;
",
    );
    has(index, "    WidgetsEntriesCursor as WidgetsEntriesCursor,\n");
}

#[test]
fn the_readme_shows_auto_pagination() {
    for layout in [Layout::Typescript, Layout::Node] {
        let f = files(layout);
        has(
            file(&f, "README.md"),
            "## Auto-pagination

List methods in the Acme API are paginated.
You can use the `for await … of` syntax to iterate through items across all pages:

```ts
async function fetchAllWidgets(params) {
  const allWidgets = [];
  // Automatically fetches more pages as needed.
  for await (const widget of client.widgets.list({ environment: 'development' })) {
    allWidgets.push(widget);
  }
  return allWidgets;
}
```

Alternatively, you can request a single page at a time:

```ts
let page = await client.widgets.list({ environment: 'development' });
for (const widget of page.entries) {
  console.log(widget);
}
",
        );
    }
}
