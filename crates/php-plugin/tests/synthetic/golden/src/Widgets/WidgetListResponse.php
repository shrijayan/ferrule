<?php

declare(strict_types=1);

namespace Acme\Widgets;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;

/**
 * @phpstan-import-type WidgetShape from \Acme\Widgets\Widget
 *
 * @phpstan-type WidgetListResponseShape = array{
 *   data: list<Widget|WidgetShape>, nextCursor?: string|null
 * }
 */
final class WidgetListResponse implements BaseModel
{
    /** @use SdkModel<WidgetListResponseShape> */
    use SdkModel;

    /** @var list<Widget> $data */
    #[Required(list: Widget::class)]
    public array $data;

    #[Optional('next_cursor', nullable: true)]
    public ?string $nextCursor;

    /**
     * `new WidgetListResponse()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * WidgetListResponse::with(data: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new WidgetListResponse)->withData(...)
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
     * @param list<Widget|WidgetShape> $data
     */
    public static function with(array $data, ?string $nextCursor = null): self
    {
        $self = new self;

        $self['data'] = $data;

        null !== $nextCursor && $self['nextCursor'] = $nextCursor;

        return $self;
    }

    /**
     * @param list<Widget|WidgetShape> $data
     */
    public function withData(array $data): self
    {
        $self = clone $this;
        $self['data'] = $data;

        return $self;
    }

    public function withNextCursor(?string $nextCursor): self
    {
        $self = clone $this;
        $self['nextCursor'] = $nextCursor;

        return $self;
    }
}
