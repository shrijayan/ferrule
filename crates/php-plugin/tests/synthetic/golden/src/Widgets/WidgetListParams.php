<?php

declare(strict_types=1);

namespace Acme\Widgets;

use Acme\Core\Attributes\Optional;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Concerns\SdkParams;
use Acme\Core\Contracts\BaseModel;

/**
 * List widgets
 *
 * @see Acme\Services\WidgetsService::list()
 *
 * @phpstan-type WidgetListParamsShape = array{
 *   colors?: list<string>|null,
 *   createdAfter?: \DateTimeInterface|null,
 *   limit?: int|null,
 * }
 */
final class WidgetListParams implements BaseModel
{
    /** @use SdkModel<WidgetListParamsShape> */
    use SdkModel;
    use SdkParams;

    /** @var list<string>|null $colors */
    #[Optional(list: 'string')]
    public ?array $colors;

    #[Optional('created_after')]
    public ?\DateTimeInterface $createdAfter;

    #[Optional]
    public ?int $limit;

    public function __construct()
    {
        $this->initialize();
    }

    /**
     * Construct an instance from the required parameters.
     *
     * You must use named parameters to construct any parameters with a default value.
     *
     * @param list<string>|null $colors
     */
    public static function with(
        ?array $colors = null,
        ?\DateTimeInterface $createdAfter = null,
        ?int $limit = null,
    ): self
    {
        $self = new self;

        null !== $colors && $self['colors'] = $colors;
        null !== $createdAfter && $self['createdAfter'] = $createdAfter;
        null !== $limit && $self['limit'] = $limit;

        return $self;
    }

    /**
     * @param list<string> $colors
     */
    public function withColors(array $colors): self
    {
        $self = clone $this;
        $self['colors'] = $colors;

        return $self;
    }

    public function withCreatedAfter(\DateTimeInterface $createdAfter): self
    {
        $self = clone $this;
        $self['createdAfter'] = $createdAfter;

        return $self;
    }

    public function withLimit(int $limit): self
    {
        $self = clone $this;
        $self['limit'] = $limit;

        return $self;
    }
}
