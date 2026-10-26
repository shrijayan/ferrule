<?php

declare(strict_types=1);

namespace Acme\Widgets\WidgetStateResponse;

use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type BlockedShape = array{kind: 'blocked', reason: string}
 */
final class Blocked implements BaseModel
{
    /** @use SdkModel<BlockedShape> */
    use SdkModel;

    /** @var 'blocked' $kind */
    #[Required]
    public string $kind = 'blocked';

    #[Required]
    public string $reason;

    /**
     * `new Blocked()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * Blocked::with(reason: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new Blocked)->withReason(...)
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
    public static function with(string $reason): self
    {
        $self = new self;

        $self['reason'] = $reason;

        return $self;
    }

    /**
     * @param 'blocked' $kind
     */
    public function withKind(string $kind): self
    {
        $self = clone $this;
        $self['kind'] = $kind;

        return $self;
    }

    public function withReason(string $reason): self
    {
        $self = clone $this;
        $self['reason'] = $reason;

        return $self;
    }
}
