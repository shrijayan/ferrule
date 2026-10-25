// File generated from our OpenAPI spec by Ferrule. See CONTRIBUTING.md for details.

import { AcmeError } from './error';
import { FinalRequestOptions } from '../internal/request-options';
import { defaultParseResponse } from '../internal/parse';
import { type Acme } from '../client';
import { APIPromise } from './api-promise';
import { type APIResponseProps } from '../internal/parse';
import { maybeObj } from '../internal/utils/values';

export type PageRequestOptions = Pick<FinalRequestOptions, 'query' | 'headers' | 'body' | 'path' | 'method'>;

export abstract class AbstractPage<Item> implements AsyncIterable<Item> {
  #client: Acme;
  protected options: FinalRequestOptions;

  protected response: Response;
  protected body: unknown;

  constructor(client: Acme, response: Response, body: unknown, options: FinalRequestOptions) {
    this.#client = client;
    this.options = options;
    this.response = response;
    this.body = body;
  }

  abstract nextPageRequestOptions(): PageRequestOptions | null;

  abstract getPaginatedItems(): Item[];

  hasNextPage(): boolean {
    const items = this.getPaginatedItems();
    if (!items.length) return false;
    return this.nextPageRequestOptions() != null;
  }

  async getNextPage(): Promise<this> {
    const nextOptions = this.nextPageRequestOptions();
    if (!nextOptions) {
      throw new AcmeError(
        'No next page expected; please check `.hasNextPage()` before calling `.getNextPage()`.',
      );
    }

    return await this.#client.requestAPIList(this.constructor as any, nextOptions);
  }

  async *iterPages(): AsyncGenerator<this> {
    let page: this = this;
    yield page;
    while (page.hasNextPage()) {
      page = await page.getNextPage();
      yield page;
    }
  }

  async *[Symbol.asyncIterator](): AsyncGenerator<Item> {
    for await (const page of this.iterPages()) {
      for (const item of page.getPaginatedItems()) {
        yield item;
      }
    }
  }
}

/**
 * This subclass of Promise will resolve to an instantiated Page once the request completes.
 *
 * It also implements AsyncIterable to allow auto-paginating iteration on an unawaited list call, eg:
 *
 *    for await (const item of client.items.list()) {
 *      console.log(item)
 *    }
 */
export class PagePromise<
    PageClass extends AbstractPage<Item>,
    Item = ReturnType<PageClass['getPaginatedItems']>[number],
  >
  extends APIPromise<PageClass>
  implements AsyncIterable<Item>
{
  constructor(
    client: Acme,
    request: Promise<APIResponseProps>,
    Page: new (...args: ConstructorParameters<typeof AbstractPage>) => PageClass,
  ) {
    super(
      client,
      request,
      async (client, props) =>
        new Page(client, props.response, await defaultParseResponse(client, props), props.options),
    );
  }

  /**
   * Allow auto-paginating iteration on an unawaited list call, eg:
   *
   *    for await (const item of client.items.list()) {
   *      console.log(item)
   *    }
   */
  async *[Symbol.asyncIterator](): AsyncGenerator<Item> {
    const page = await this;
    for await (const item of page) {
      yield item;
    }
  }
}

export interface EntriesCursorResponse<Item> {
  entries: Array<Item>;

  page_info: EntriesCursorResponse.PageInfo;
}

export namespace EntriesCursorResponse {
  export interface PageInfo {
    after?: string;
  }
}

export interface EntriesCursorParams {
  after?: string;

  before?: string;

  limit?: number;
}

export class EntriesCursor<Item> extends AbstractPage<Item> implements EntriesCursorResponse<Item> {
  entries: Array<Item>;

  page_info: EntriesCursorResponse.PageInfo;

  constructor(
    client: Acme,
    response: Response,
    body: EntriesCursorResponse<Item>,
    options: FinalRequestOptions,
  ) {
    super(client, response, body, options);

    this.entries = body.entries || [];
    this.page_info = body.page_info || {};
  }

  getPaginatedItems(): Item[] {
    return this.entries ?? [];
  }

  nextPageRequestOptions(): PageRequestOptions | null {
    const cursor = this.page_info?.after;
    if (!cursor) {
      return null;
    }

    return {
      ...this.options,
      query: {
        ...maybeObj(this.options.query),
        after: cursor,
      },
    };
  }
}

export interface PageResponse<Item> {
  total: number;

  items: Array<Item>;

  page: number;

  size: number;

  pages: number;
}

export interface PageParams {
  /**
   * The page number
   */
  page?: number;

  /**
   * The number of items in a page
   */
  size?: number;
}

export class Page<Item> extends AbstractPage<Item> implements PageResponse<Item> {
  total: number;

  items: Array<Item>;

  page: number;

  size: number;

  pages: number;

  constructor(client: Acme, response: Response, body: PageResponse<Item>, options: FinalRequestOptions) {
    super(client, response, body, options);

    this.total = body.total || 0;
    this.items = body.items || [];
    this.page = body.page || 0;
    this.size = body.size || 0;
    this.pages = body.pages || 0;
  }

  getPaginatedItems(): Item[] {
    return this.items ?? [];
  }

  nextPageRequestOptions(): PageRequestOptions | null {
    const currentPage = this.page;

    if (currentPage >= this.pages) {
      return null;
    }

    return {
      ...this.options,
      query: {
        ...maybeObj(this.options.query),
        page: currentPage + 1,
      },
    };
  }
}
