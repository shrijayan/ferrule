<?php

declare(strict_types=1);

namespace Acme\Widgets;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Concerns\SdkParams;
use Acme\Core\Contracts\BaseModel;
use Acme\Widgets\WidgetCreateParams\Color;
use Acme\Widgets\WidgetCreateParams\Dimensions;

/**
 * Creates a widget with the given name and parts. The SKU is assigned by the server.
 *
 * @see Acme\Services\WidgetsService::create()
 *
 * @phpstan-import-type DimensionsShape from \Acme\Widgets\WidgetCreateParams\Dimensions
 *
 * @phpstan-type WidgetCreateParamsShape = array{
 *   name: string,
 *   color?: null|Color|value-of<Color>,
 *   dimensions?: null|Dimensions|DimensionsShape,
 *   tags?: list<string>|null,
 * }
 */
final class WidgetCreateParams implements BaseModel
{
    /** @use SdkModel<WidgetCreateParamsShape> */
    use SdkModel;
    use SdkParams;

    /**
     * The widget's display name.
     */
    #[Required]
    public string $name;

    /** @var value-of<Color>|null $color */
    #[Optional(enum: Color::class)]
    public ?string $color;

    #[Optional]
    public ?Dimensions $dimensions;

    /** @var list<string>|null $tags */
    #[Optional(list: 'string')]
    public ?array $tags;

    /**
     * `new WidgetCreateParams()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * WidgetCreateParams::with(name: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new WidgetCreateParams)->withName(...)
     * ```
     */
    public function __construct()
    {
        $this->initialize();
    }

    /**
     * Construct an instance from the required parameters.
     *
     * You must use named parameters to construct any parameters with a default value.
     *
     * @param Color|value-of<Color>|null $color
     * @param Dimensions|DimensionsShape|null $dimensions
     * @param list<string>|null $tags
     */
    public static function with(
        string $name,
        Color|string|null $color = null,
        Dimensions|array|null $dimensions = null,
        ?array $tags = null,
    ): self
    {
        $self = new self;

        $self['name'] = $name;

        null !== $color && $self['color'] = $color;
        null !== $dimensions && $self['dimensions'] = $dimensions;
        null !== $tags && $self['tags'] = $tags;

        return $self;
    }

    /**
     * The widget's display name.
     */
    public function withName(string $name): self
    {
        $self = clone $this;
        $self['name'] = $name;

        return $self;
    }

    /**
     * @param Color|value-of<Color> $color
     */
    public function withColor(Color|string $color): self
    {
        $self = clone $this;
        $self['color'] = $color;

        return $self;
    }

    /**
     * @param Dimensions|DimensionsShape $dimensions
     */
    public function withDimensions(Dimensions|array $dimensions): self
    {
        $self = clone $this;
        $self['dimensions'] = $dimensions;

        return $self;
    }

    /**
     * @param list<string> $tags
     */
    public function withTags(array $tags): self
    {
        $self = clone $this;
        $self['tags'] = $tags;

        return $self;
    }
}
