<?php

declare(strict_types=1);

namespace Acme;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type LabelItemShape = array{text: string, color?: string|null}
 */
final class LabelItem implements BaseModel
{
    /** @use SdkModel<LabelItemShape> */
    use SdkModel;

    #[Required]
    public string $text;

    #[Optional]
    public ?string $color;

    /**
     * `new LabelItem()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * LabelItem::with(text: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new LabelItem)->withText(...)
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
    public static function with(string $text, ?string $color = null): self
    {
        $self = new self;

        $self['text'] = $text;

        null !== $color && $self['color'] = $color;

        return $self;
    }

    public function withText(string $text): self
    {
        $self = clone $this;
        $self['text'] = $text;

        return $self;
    }

    public function withColor(string $color): self
    {
        $self = clone $this;
        $self['color'] = $color;

        return $self;
    }
}
