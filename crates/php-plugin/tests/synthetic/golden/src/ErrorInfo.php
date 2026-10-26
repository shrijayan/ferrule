<?php

declare(strict_types=1);

namespace Acme;

use Acme\Core\Attributes\Optional;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-type ErrorInfoShape = array{code?: string|null, message?: string|null}
 */
final class ErrorInfo implements BaseModel
{
    /** @use SdkModel<ErrorInfoShape> */
    use SdkModel;

    #[Optional]
    public ?string $code;

    #[Optional]
    public ?string $message;

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
        ?string $code = null, ?string $message = null
    ): self
    {
        $self = new self;

        null !== $code && $self['code'] = $code;
        null !== $message && $self['message'] = $message;

        return $self;
    }

    public function withCode(string $code): self
    {
        $self = clone $this;
        $self['code'] = $code;

        return $self;
    }

    public function withMessage(string $message): self
    {
        $self = clone $this;
        $self['message'] = $message;

        return $self;
    }
}
