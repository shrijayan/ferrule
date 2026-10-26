<?php

declare(strict_types=1);

namespace Acme\Widgets\Parts;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;
use Acme\ErrorInfo;

/**
 * @phpstan-import-type ErrorInfoShape from \Acme\ErrorInfo
 *
 * @phpstan-type PartRetrieveResponseShape = array{
 *   id: string, sku: string, error?: null|ErrorInfo|ErrorInfoShape
 * }
 */
final class PartRetrieveResponse implements BaseModel
{
    /** @use SdkModel<PartRetrieveResponseShape> */
    use SdkModel;

    #[Required]
    public string $id;

    /**
     * The part's stock keeping unit.
     */
    #[Required]
    public string $sku;

    #[Optional]
    public ?ErrorInfo $error;

    /**
     * `new PartRetrieveResponse()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * PartRetrieveResponse::with(id: ..., sku: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new PartRetrieveResponse)->withID(...)->withSKU(...)
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
     * @param ErrorInfo|ErrorInfoShape|null $error
     */
    public static function with(
        string $id, string $sku, ErrorInfo|array|null $error = null
    ): self
    {
        $self = new self;

        $self['id'] = $id;
        $self['sku'] = $sku;

        null !== $error && $self['error'] = $error;

        return $self;
    }

    public function withID(string $id): self
    {
        $self = clone $this;
        $self['id'] = $id;

        return $self;
    }

    /**
     * The part's stock keeping unit.
     */
    public function withSKU(string $sku): self
    {
        $self = clone $this;
        $self['sku'] = $sku;

        return $self;
    }

    /**
     * @param ErrorInfo|ErrorInfoShape $error
     */
    public function withError(ErrorInfo|array $error): self
    {
        $self = clone $this;
        $self['error'] = $error;

        return $self;
    }
}
