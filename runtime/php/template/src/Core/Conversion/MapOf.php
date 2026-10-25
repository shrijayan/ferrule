<?php

declare(strict_types=1);

namespace __FerruleNs__\Core\Conversion;

use __FerruleNs__\Core\Conversion\Concerns\ArrayOf;
use __FerruleNs__\Core\Conversion\Contracts\Converter;

/**
 * @internal
 */
final class MapOf implements Converter
{
    use ArrayOf;
}
