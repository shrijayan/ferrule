<?php

declare(strict_types=1);

namespace Acme\Widgets\WidgetCreateParams;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type DimensionsShape = array{width: float, height?: float|null}
 */
final class Dimensions implements BaseModel
{
    /** @use SdkModel<DimensionsShape> */
    use SdkModel;

    #[Required]
    public float $width;

    #[Optional]
    public ?float $height;

    /**
     * `new Dimensions()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * Dimensions::with(width: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new Dimensions)->withWidth(...)
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
     */
    public static function with(float $width, ?float $height = null): self
    {
        $self = new self;

        $self['width'] = $width;

        null !== $height && $self['height'] = $height;

        return $self;
    }

    public function withWidth(float $width): self
    {
        $self = clone $this;
        $self['width'] = $width;

        return $self;
    }

    public function withHeight(float $height): self
    {
        $self = clone $this;
        $self['height'] = $height;

        return $self;
    }
}
