import { describe, expect, it } from 'vitest';
import {
  buildStreaming,
  readOnEvent,
  resolveEventModel,
  type StreamingInput,
} from '../../src/extensions/streaming.ts';

const spec = {
  components: {
    requestBodies: {
      Chat: {
        content: {
          'application/json': {
            schema: { type: 'object', properties: { stream: { type: 'boolean' } } },
          },
        },
      },
    },
  },
};

const config = {
  resources: {
    chat: {
      subresources: {
        completions: { models: { chunk: '#/components/schemas/Chunk', other: { x: 1 } } },
      },
    },
    events: { models: { event: 'Event' } },
  },
};

function input(streaming: unknown, responses: Record<string, unknown> = {}): StreamingInput {
  return {
    spec,
    config,
    op: { requestBody: { $ref: '#/components/requestBodies/Chat' }, responses },
    streaming,
    pointer: '/resources/chat/methods/create',
    onEvent: [],
  };
}

const sse = { '200': { content: { 'text/event-stream': {}, 'application/json': {} } } };
const jsonl = { '200': { content: { 'application/x-ndjson': {} } } };

describe('x-ferrule-streaming', () => {
  it('reads a method streaming block', () => {
    const { streaming, diagnostics } = buildStreaming(
      input({
        param_discriminator: 'stream',
        stream_event_model: 'chat.completions.chunk',
        params_type_name: 'completion_create_params',
        method_name: 'stream',
        synthesize_event_and_data_properties: true,
      }),
    );
    expect(diagnostics).toEqual([]);
    expect(streaming).toEqual({
      format: 'sse',
      paramDiscriminator: 'stream',
      eventModel: 'chat.completions.chunk',
      eventSchema: 'Chunk',
      paramsTypeName: 'completion_create_params',
      methodName: 'stream',
      synthesizeEventAndData: true,
      onEvent: [],
    });
  });

  it('infers the format from the response content type unless written', () => {
    const always = { param_discriminator: null };
    expect(buildStreaming(input(always, sse)).streaming?.format).toBe('sse');
    expect(buildStreaming(input(always, jsonl)).streaming?.format).toBe('jsonl');
    expect(buildStreaming(input({ ...always, type: 'jsonl' }, sse)).streaming?.format).toBe(
      'jsonl',
    );
    expect(buildStreaming(input({ ...always, type: 'sse' }, jsonl)).streaming?.format).toBe('sse');
    const { streaming, diagnostics } = buildStreaming(input(always));
    expect(streaming).toMatchObject({ format: 'sse', paramDiscriminator: null });
    expect(diagnostics).toEqual([]);
  });

  it('is absent when not configured or skipped for python', () => {
    expect(buildStreaming(input(undefined)).streaming).toBeUndefined();
    expect(buildStreaming(input(true)).streaming).toBeUndefined();
    expect(
      buildStreaming(input({ param_discriminator: null, skip: ['python'] })).streaming,
    ).toBeUndefined();
    expect(
      buildStreaming(input({ param_discriminator: null, skip: ['go'] })).streaming,
    ).toBeDefined();
  });

  it('warns about a missing or unknown discriminator and an unresolved event model', () => {
    const missing = buildStreaming(input({ stream_event_model: 'chat.nope' }));
    expect(missing.streaming?.paramDiscriminator).toBeNull();
    expect(missing.diagnostics.map((d) => [d.code, d.pointer])).toEqual([
      ['FER1401', '/resources/chat/methods/create/streaming'],
      ['FER1403', '/resources/chat/methods/create/streaming/stream_event_model'],
    ]);
    const unknown = buildStreaming({
      ...input({ param_discriminator: 'go' }),
      op: { requestBody: 'bad' },
    });
    expect(unknown.diagnostics.map((d) => [d.code, d.severity, d.pointer])).toEqual([
      ['FER1402', 'warning', '/resources/chat/methods/create/streaming/param_discriminator'],
    ]);
  });
});

describe('resolveEventModel', () => {
  it.each([
    ['chat.completions.chunk', 'Chunk'],
    ['events.event', 'Event'],
    ['chat.completions.other', undefined],
    ['chat.missing.chunk', undefined],
    ['nope.event', undefined],
    ['event', undefined],
  ])('%s -> %s', (written, schema) => {
    expect(resolveEventModel(config, written)).toBe(schema);
  });
});

describe('readOnEvent', () => {
  it('normalizes kinds and keeps config order', () => {
    expect(
      readOnEvent({
        streaming: {
          on_event: [
            { data_starts_with: '[DONE]', handle: 'done' },
            { event_type: null, handle: 'yield' },
            { kind: 'event', event_type: ['error', 3], handle: 'error', error_property: 'msg' },
            { event_type: 'ping', handle: 'continue' },
            { kind: 'fallthrough', handle: 'break' },
            { kind: 'data', data_starts_with: 'x', handle: 'nope' },
            'junk',
          ],
        },
      }),
    ).toEqual([
      { kind: 'data', handle: 'done', dataStartsWith: '[DONE]' },
      { kind: 'event', handle: 'yield', eventType: null },
      { kind: 'event', handle: 'error', eventType: ['error'], errorProperty: 'msg' },
      { kind: 'event', handle: 'continue', eventType: 'ping' },
      { kind: 'fallthrough', handle: 'break' },
    ]);
    expect(readOnEvent({})).toEqual([]);
  });
});
