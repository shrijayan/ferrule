<?php

declare(strict_types=1);

namespace Acme\Widgets\WidgetStateResponse;

use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type ReadyShape = array{kind: 'ready'}
 */
final class Ready implements BaseModel
{
    /** @use SdkModel<ReadyShape> */
    use SdkModel;

    /** @var 'ready' $kind */
    #[Required]
    public string $kind = 'ready';

    public function __construct()
    {
        $this->initialize();
    }

    /**
     * Construct an instance from the required parameters.
     *
     * You must use named parameters to construct any parameters with a default value.
     */
    public static function with(): self
    {
        $self = new self;

        return $self;
    }

    /**
     * @param 'ready' $kind
     */
    public function withKind(string $kind): self
    {
        $self = clone $this;
        $self['kind'] = $kind;

        return $self;
    }
}
