/**
 * What the project-file emitters need, as a narrow type of its own. Phase 3 task C defines it
 * here so the emitters don't wait on SdkModel; the lead wires it from SdkModel and the config
 * (mapping in the task C report). Everything is already a Python or text spelling.
 */
import type { HttpVerb, SdkType } from '../../model/index.ts';

/** Config editions the templates were extracted for (OneBusAway: Rye, Python 3.9). */
export type PythonEdition = 'python.2025-10-08';

/** One request used as an example in README.md, or as the retry probe in tests/test_client.py. */
export interface ExampleRequest {
  /** Dotted resource accessor from the client, such as `current_time` or `chat.completions`. */
  readonly accessor: string;
  /** Python method name, such as `retrieve`. */
  readonly method: string;
  /** Python keyword arguments, already rendered, such as `stop_id="1_75403"`; empty for none. */
  readonly args: string;
  readonly httpMethod: HttpVerb;
  /** The path as respx matches it, such as `/api/where/current-time.json`. */
  readonly path: string;
}

export type ParamLocation = 'path' | 'query' | 'body';

export interface FilesParam {
  readonly pyName: string;
  readonly wireName: string;
  readonly required: boolean;
  readonly location: ParamLocation;
  readonly type: SdkType;
  /** The spec's `example`, when it has one; strings otherwise default to the wire name. */
  readonly example?: unknown;
  /** The schema's `minimum`/`maximum`: a number example is the minimum, else the maximum, else 0. */
  readonly minimum?: number;
  readonly maximum?: number;
}

/** A Python name and the dotted module (under the package) that declares it. */
export interface PyName {
  readonly name: string;
  /** Such as `types.trips_for_location_list_response` or `types.shared.response_wrapper`. */
  readonly module: string;
}

export interface FilesMethod {
  readonly name: string;
  readonly httpMethod: HttpVerb;
  /** The path as written in the spec, such as `/api/where/agency/{agencyID}.json`. */
  readonly path: string;
  /** Path parameters first, in path order, then the rest. */
  readonly params: readonly FilesParam[];
  /** Dotted module of the `*_params.py` file; absent when the method has no params class. */
  readonly paramsModule?: string;
  /** The return type; absent for `None`. */
  readonly response?: PyName;
}

export interface FilesResource {
  /** Python attribute name, such as `trip_details`. */
  readonly name: string;
  /** Class segment, such as `TripDetails` or `StopIDsForAgency` (api.md heading, test class). */
  readonly title: string;
  /** Local variable the tests and README use for a result, such as `trip_detail`. */
  readonly variable: string;
  /** Dotted module of the resource class, such as `resources.trip_details`. */
  readonly module: string;
  /** Names api.md lists under "Types:", in order, from module `typesModule`. */
  readonly types: readonly string[];
  /** Such as `types` or `types.chat`. */
  readonly typesModule: string;
  readonly methods: readonly FilesMethod[];
  readonly subresources: readonly FilesResource[];
}

export interface ProjectFilesInput {
  readonly edition: PythonEdition;
  /** Python import name, such as `onebusaway`. */
  readonly packageName: string;
  /** PyPI distribution name, such as `onebusaway`. */
  readonly distributionName: string;
  /** Client class, such as `OnebusawaySDK`. */
  readonly clientClass: string;
  /** Prefix of the SDK's own env vars (`_BASE_URL`, `_LOG`), such as `ONEBUSAWAY_SDK`. */
  readonly envPrefix: string;
  /** GitHub `owner/repo`, such as `OneBusAway/python-sdk`. */
  readonly repo: string;
  /** Package version, such as `1.30.0`. */
  readonly version: string;
  /** Year in the LICENSE copyright line (an input: the plugin has no clock). */
  readonly copyrightYear: number;
  readonly organization: {
    /** `organization.name`, such as `onebusaway-sdk`. */
    readonly name: string;
    /** Display name, such as `Onebusaway SDK`. */
    readonly title: string;
    readonly contact: string;
    readonly docsUrl: string;
  };
  /** First heading of README.md without " Python API library", such as `OneBusAway SDK`. */
  readonly readmeTitle: string;
  /** The API key client option: env var it reads and the query parameter it is sent as. */
  readonly auth: { readonly kind: 'api_key_query'; readonly env: string; readonly param: string };
  readonly query: {
    readonly arrayFormat: 'comma' | 'repeat' | 'brackets' | 'indices';
    readonly objectFormat: 'brackets' | 'dots';
  };
  readonly stats: {
    readonly openapiSpecUrl: string;
    readonly openapiSpecHash: string;
    readonly configHash: string;
  };
  readonly readmeExample: ExampleRequest;
  readonly retryProbe: ExampleRequest;
  /** Names of `types.shared`, for the "Shared Types" section of api.md. */
  readonly sharedTypes: readonly string[];
  readonly resources: readonly FilesResource[];
}
