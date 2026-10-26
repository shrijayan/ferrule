<?php

declare(strict_types=1);

namespace Acme\Widgets;

use Acme\Core\Concerns\SdkUnion;
use Acme\Core\Conversion\Contracts\Converter;
use Acme\Core\Conversion\Contracts\ConverterSource;
use Acme\Widgets\WidgetStateResponse\Blocked;
use Acme\Widgets\WidgetStateResponse\Ready;

/**
 * @phpstan-import-type ReadyShape from \Acme\Widgets\WidgetStateResponse\Ready
 * @phpstan-import-type BlockedShape from \Acme\Widgets\WidgetStateResponse\Blocked
 *
 * @phpstan-type WidgetStateResponseVariants = Ready|Blocked
 * @phpstan-type WidgetStateResponseShape = WidgetStateResponseVariants|ReadyShape|BlockedShape
 */
final class WidgetStateResponse implements ConverterSource
{
    use SdkUnion;

    public static function discriminator(): string
    {
        return 'kind';
    }

    /**
     * @return list<string|Converter|ConverterSource>|array<string,string|Converter|ConverterSource>
     */
    public static function variants(): array
    {
        return ['ready' => Ready::class, 'blocked' => Blocked::class];
    }
}
