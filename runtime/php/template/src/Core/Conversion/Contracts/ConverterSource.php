<?php

declare(strict_types=1);

namespace __FerruleNs__\Core\Conversion\Contracts;

/**
 * @internal
 */
interface ConverterSource
{
    public static function converter(): Converter;
}
