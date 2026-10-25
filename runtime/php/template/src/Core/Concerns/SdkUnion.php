<?php

declare(strict_types=1);

namespace __FerruleNs__\Core\Concerns;

use __FerruleNs__\Core\Conversion\Contracts\Converter;
use __FerruleNs__\Core\Conversion\Contracts\ConverterSource;
use __FerruleNs__\Core\Conversion\UnionOf;

/**
 * @internal
 */
trait SdkUnion
{
    private static Converter $converter;

    public static function discriminator(): ?string
    {
        return null;
    }

    /**
     * @return array<string, Converter|ConverterSource|string>|list<Converter|ConverterSource|string>
     */
    public static function variants(): array
    {
        return [];
    }

    public static function converter(): Converter
    {
        if (isset(static::$converter)) {
            return static::$converter;
        }

        return static::$converter = new UnionOf(discriminator: static::discriminator(), variants: static::variants());
    }
}
