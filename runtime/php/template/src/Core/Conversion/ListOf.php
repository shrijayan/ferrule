<?php

declare(strict_types=1);

namespace __FerruleNs__\Core\Conversion;

use __FerruleNs__\Core\Conversion\Concerns\ArrayOf;
use __FerruleNs__\Core\Conversion\Contracts\Converter;

/**
 * @internal
 */
final class ListOf implements Converter
{
    use ArrayOf;

    // @phpstan-ignore-next-line missingType.iterableValue
    private function empty(): array|object
    {
        return [];
    }
}
