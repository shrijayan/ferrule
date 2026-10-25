<?php

declare(strict_types=1);

namespace __FerruleNs__\Core\Conversion\Contracts;

use __FerruleNs__\Core\Conversion\CoerceState;
use __FerruleNs__\Core\Conversion\DumpState;

/**
 * @internal
 */
interface Converter
{
    /**
     * @internal
     */
    public function coerce(mixed $value, CoerceState $state): mixed;

    /**
     * @internal
     */
    public function dump(mixed $value, DumpState $state): mixed;
}
