<?php

declare(strict_types=1);

namespace Acme\Widgets\Widget;

use Acme\Core\Attributes\Optional;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type PartShape = array{label?: string|null, partNumber?: int|null}
 */
final class Part implements BaseModel
{
    /** @use SdkModel<PartShape> */
    use SdkModel;

    #[Optional]
    public ?string $label;

    #[Optional('part_number')]
    public ?int $partNumber;

    public function __construct()
    {
        $this->initialize();
    }

    /**
     * Construct an instance from the required parameters.
     *
     * You must use named parameters to construct any parameters with a default value.
     */
    public static function with(
        ?string $label = null, ?int $partNumber = null
    ): self
    {
        $self = new self;

        null !== $label && $self['label'] = $label;
        null !== $partNumber && $self['partNumber'] = $partNumber;

        return $self;
    }

    public function withLabel(string $label): self
    {
        $self = clone $this;
        $self['label'] = $label;

        return $self;
    }

    public function withPartNumber(int $partNumber): self
    {
        $self = clone $this;
        $self['partNumber'] = $partNumber;

        return $self;
    }
}
