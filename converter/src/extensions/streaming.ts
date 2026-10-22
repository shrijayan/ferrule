/**
 * `x-ferrule-streaming` (ADR 0003): a method's `streaming:` block in stainless.yml, with the
 * top-level `streaming.on_event` handlers. Pure.
 */
import type { Diagnostic } from '../diagnostic.ts';
import {
  arr,
  at,
  bool,
  deref,
  isObject,
  type Json,
  obj,
  propertyNames,
  str,
  successResponse,
} from './json.ts';

export type StreamFormat = 'sse' | 'jsonl';
export type EventHandle = 'break' | 'continue' | 'done' | 'yield' | 'error';

/** A top-level `streaming.on_event` entry, in config order. */
export interface FerruleOnEvent {
  readonly kind: 'event' | 'fallthrough' | 'data';
  readonly handle: EventHandle;
  /** `event` entries: the SSE event name(s); `null` matches events without a name. */
  readonly eventType?: string | readonly string[] | null;
  readonly errorProperty?: string;
  readonly dataStartsWith?: string;
}

export interface FerruleStreaming {
  readonly format: StreamFormat;
  /** Request body param that turns streaming on; `null` when the method always streams. */
  readonly paramDiscriminator: string | null;
  /** `stream_event_model` as written (`$resource.$model`). */
  readonly eventModel?: string;
  /** The component schema `eventModel` names, when it resolves. */
  readonly eventSchema?: string;
  readonly paramsTypeName?: string;
  readonly methodName?: string;
  readonly synthesizeEventAndData: boolean;
  readonly onEvent: readonly FerruleOnEvent[];
}

const JSONL_TYPES = new Set([
  'application/x-ndjson',
  'application/jsonl',
  'application/jsonlines',
  'application/x-jsonlines',
]);
const HANDLES: readonly EventHandle[] = ['break', 'continue', 'done', 'yield', 'error'];

/** `type`, else inferred from the 2xx content types, else `sse` (public schema default). */
function format(spec: Json, op: Json, written: unknown): StreamFormat {
  if (written === 'sse' || written === 'jsonl') return written;
  const resp = successResponse(spec, op);
  const types = Object.keys(resp === undefined ? {} : obj(resp, 'content'));
  if (types.includes('text/event-stream')) return 'sse';
  return types.some((t) => JSONL_TYPES.has(t)) ? 'jsonl' : 'sse';
}

/** Read top-level `streaming.on_event`; entries with an unknown handle are dropped. */
export function readOnEvent(config: Json): FerruleOnEvent[] {
  return arr(obj(config, 'streaming'), 'on_event').flatMap((raw): FerruleOnEvent[] => {
    const e = isObject(raw) ? raw : {};
    const handle = at(e, 'handle');
    if (!HANDLES.includes(handle as EventHandle)) return [];
    const dataStartsWith = str(e, 'data_starts_with');
    const errorProperty = str(e, 'error_property');
    const written = str(e, 'kind');
    const kind =
      written === 'fallthrough' || written === 'data' || written === 'event'
        ? written
        : dataStartsWith === undefined
          ? 'event'
          : 'data';
    const eventType = at(e, 'event_type');
    return [
      {
        kind,
        handle: handle as EventHandle,
        ...(kind === 'event'
          ? {
              eventType:
                typeof eventType === 'string' || eventType === null
                  ? eventType
                  : arr(e, 'event_type').filter((t) => typeof t === 'string'),
            }
          : {}),
        ...(errorProperty === undefined ? {} : { errorProperty }),
        ...(dataStartsWith === undefined ? {} : { dataStartsWith }),
      },
    ];
  });
}

/** Schema name of `$resource.$model` (sub-resources joined by dots), or undefined. */
export function resolveEventModel(config: Json, written: string): string | undefined {
  const segs = written.split('.');
  const key = segs.pop();
  let resources = obj(config, 'resources');
  let resource: Json | undefined;
  for (const [i, seg] of segs.entries()) {
    resource = isObject(resources[seg]) ? (resources[seg] as Json) : undefined;
    if (resource === undefined) return undefined;
    if (i < segs.length - 1) resources = obj(resource, 'subresources');
  }
  const model =
    resource === undefined || key === undefined ? undefined : obj(resource, 'models')[key];
  return typeof model === 'string' ? model.replace(/^#\/components\/schemas\//, '') : undefined;
}

export interface StreamingInput {
  readonly spec: Json;
  readonly config: Json;
  readonly op: Json;
  /** The method's `streaming` value. */
  readonly streaming: unknown;
  /** JSON pointer of the method in the config. */
  readonly pointer: string;
  readonly onEvent: readonly FerruleOnEvent[];
}

/** Build `x-ferrule-streaming`, or undefined when not configured or skipped for Python. */
export function buildStreaming(input: StreamingInput): {
  streaming: FerruleStreaming | undefined;
  diagnostics: Diagnostic[];
} {
  const { spec, config, op, pointer, onEvent } = input;
  const diagnostics: Diagnostic[] = [];
  if (!isObject(input.streaming)) return { streaming: undefined, diagnostics };
  const s = input.streaming;
  if (arr(s, 'skip').includes('python')) return { streaming: undefined, diagnostics };

  const rawDiscriminator = at(s, 'param_discriminator');
  const paramDiscriminator = typeof rawDiscriminator === 'string' ? rawDiscriminator : null;
  if (!Object.hasOwn(s, 'param_discriminator')) {
    diagnostics.push({
      code: 'FER1401',
      severity: 'warning',
      message: 'streaming has no param_discriminator; the method is treated as always streaming.',
      pointer: `${pointer}/streaming`,
      suggestion: 'Set param_discriminator to the request param that enables streaming, or null.',
    });
  } else if (paramDiscriminator !== null) {
    const body = deref(spec, at(op, 'requestBody'));
    const schema = at(
      obj(obj(isObject(body) ? body : {}, 'content'), 'application/json'),
      'schema',
    );
    if (!propertyNames(spec, schema).has(paramDiscriminator)) {
      diagnostics.push({
        code: 'FER1402',
        severity: 'warning',
        message: `param_discriminator ${paramDiscriminator} is not a JSON request body property.`,
        pointer: `${pointer}/streaming/param_discriminator`,
        suggestion: 'Name a boolean body property such as `stream`, or use null.',
      });
    }
  }

  const eventModel = str(s, 'stream_event_model');
  const eventSchema = eventModel === undefined ? undefined : resolveEventModel(config, eventModel);
  if (eventModel !== undefined && eventSchema === undefined) {
    diagnostics.push({
      code: 'FER1403',
      severity: 'warning',
      message: `stream_event_model ${eventModel} is not a model of a configured resource.`,
      pointer: `${pointer}/streaming/stream_event_model`,
      suggestion: "Write it as `$resource.$model`, naming a key under that resource's models.",
    });
  }
  const paramsTypeName = str(s, 'params_type_name');
  const methodName = str(s, 'method_name');
  return {
    streaming: {
      format: format(spec, op, at(s, 'type')),
      paramDiscriminator,
      ...(eventModel === undefined ? {} : { eventModel }),
      ...(eventSchema === undefined ? {} : { eventSchema }),
      ...(paramsTypeName === undefined ? {} : { paramsTypeName }),
      ...(methodName === undefined ? {} : { methodName }),
      synthesizeEventAndData: bool(s, 'synthesize_event_and_data_properties') ?? false,
      onEvent,
    },
    diagnostics,
  };
}
