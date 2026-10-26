<?php

declare(strict_types=1);

namespace Acme\Widgets\Parts;

use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Concerns\SdkParams;
use Acme\Core\Contracts\BaseModel;

/**
 * Retrieve one part of a widget
 *
 * @see Acme\Services\Widgets\PartsService::retrieve()
 *
 * @phpstan-type PartRetrieveParamsShape = array{widgetID: string}
 */
final class PartRetrieveParams implements BaseModel
{
    /** @use SdkModel<PartRetrieveParamsShape> */
    use SdkModel;
    use SdkParams;

    #[Required]
    public string $widgetID;

    /**
     * `new PartRetrieveParams()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * PartRetrieveParams::with(widgetID: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new PartRetrieveParams)->withWidgetID(...)
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
    public static function with(string $widgetID): self
    {
        $self = new self;

        $self['widgetID'] = $widgetID;

        return $self;
    }

    public function withWidgetID(string $widgetID): self
    {
        $self = clone $this;
        $self['widgetID'] = $widgetID;

        return $self;
    }
}
