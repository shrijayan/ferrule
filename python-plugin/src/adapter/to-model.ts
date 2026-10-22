import type { SdkClient } from '../model/index.ts';
import { splitWords } from '../naming.ts';

/**
 * Minimal input shape the adapter accepts in phase 0. Replaced by Forge's IR types once
 * Forge is installable; this directory stays the only code that knows them.
 */
export interface AdapterInput {
  readonly title: string;
  readonly packageName: string;
  readonly baseUrl: string;
}

export function toModel(input: AdapterInput): SdkClient {
  return {
    name: splitWords(input.title),
    packageName: input.packageName,
    baseUrl: input.baseUrl,
    resources: [],
  };
}
