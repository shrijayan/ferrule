<?php

declare(strict_types=1);

namespace Acme\Widgets;

use Acme\Core\Attributes\Optional;
use Acme\Core\Attributes\Required;
use Acme\Core\Concerns\SdkModel;
use Acme\Core\Contracts\BaseModel;
use Acme\LabelItem;
use Acme\Widgets\Widget\Part;
use Acme\Widgets\Widget\Status;

/**
 * A widget in the catalogue.
 *
 * @phpstan-import-type LabelItemShape from \Acme\LabelItem
 * @phpstan-import-type PartShape from \Acme\Widgets\Widget\Part
 *
 * @phpstan-type WidgetShape = array{
 *   id: string,
 *   createdAt: \DateTimeInterface,
 *   name: string,
 *   status: Status|value-of<Status>,
 *   homepageURL?: string|null,
 *   labels?: array<string,LabelItem|LabelItemShape>|null,
 *   parts?: list<Part|PartShape>|null,
 *   sku?: string|null,
 *   weightKg?: float|null,
 * }
 */
final class Widget implements BaseModel
{
    /** @use SdkModel<WidgetShape> */
    use SdkModel;

    #[Required]
    public string $id;

    #[Required('created_at')]
    public \DateTimeInterface $createdAt;

    #[Required]
    public string $name;

    /**
     * Where the widget is in its life cycle.
     *
     * @var value-of<Status> $status
     */
    #[Required(enum: Status::class)]
    public string $status;

    #[Optional('homepage_url')]
    public ?string $homepageURL;

    /**
     * Labels by language.
     *
     * @var array<string,LabelItem>|null $labels
     */
    #[Optional(map: LabelItem::class)]
    public ?array $labels;

    /** @var list<Part>|null $parts */
    #[Optional(list: Part::class)]
    public ?array $parts;

    #[Optional(nullable: true)]
    public ?string $sku;

    #[Optional('weight_kg')]
    public ?float $weightKg;

    /**
     * `new Widget()` is missing required properties by the API.
     *
     * To enforce required parameters use
     * ```
     * Widget::with(id: ..., createdAt: ..., name: ..., status: ...)
     * ```
     *
     * Otherwise ensure the following setters are called
     *
     * ```
     * (new Widget)->withID(...)->withCreatedAt(...)->withName(...)->withStatus(...)
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
     * @param Status|value-of<Status> $status
     * @param array<string,LabelItem|LabelItemShape>|null $labels
     * @param list<Part|PartShape>|null $parts
     */
    public static function with(
        string $id,
        \DateTimeInterface $createdAt,
        string $name,
        Status|string $status,
        ?string $homepageURL = null,
        ?array $labels = null,
        ?array $parts = null,
        ?string $sku = null,
        ?float $weightKg = null,
    ): self
    {
        $self = new self;

        $self['id'] = $id;
        $self['createdAt'] = $createdAt;
        $self['name'] = $name;
        $self['status'] = $status;

        null !== $homepageURL && $self['homepageURL'] = $homepageURL;
        null !== $labels && $self['labels'] = $labels;
        null !== $parts && $self['parts'] = $parts;
        null !== $sku && $self['sku'] = $sku;
        null !== $weightKg && $self['weightKg'] = $weightKg;

        return $self;
    }

    public function withID(string $id): self
    {
        $self = clone $this;
        $self['id'] = $id;

        return $self;
    }

    public function withCreatedAt(\DateTimeInterface $createdAt): self
    {
        $self = clone $this;
        $self['createdAt'] = $createdAt;

        return $self;
    }

    public function withName(string $name): self
    {
        $self = clone $this;
        $self['name'] = $name;

        return $self;
    }

    /**
     * Where the widget is in its life cycle.
     *
     * @param Status|value-of<Status> $status
     */
    public function withStatus(Status|string $status): self
    {
        $self = clone $this;
        $self['status'] = $status;

        return $self;
    }

    public function withHomepageURL(string $homepageURL): self
    {
        $self = clone $this;
        $self['homepageURL'] = $homepageURL;

        return $self;
    }

    /**
     * Labels by language.
     *
     * @param array<string,LabelItem|LabelItemShape> $labels
     */
    public function withLabels(array $labels): self
    {
        $self = clone $this;
        $self['labels'] = $labels;

        return $self;
    }

    /**
     * @param list<Part|PartShape> $parts
     */
    public function withParts(array $parts): self
    {
        $self = clone $this;
        $self['parts'] = $parts;

        return $self;
    }

    public function withSKU(?string $sku): self
    {
        $self = clone $this;
        $self['sku'] = $sku;

        return $self;
    }

    public function withWeightKg(float $weightKg): self
    {
        $self = clone $this;
        $self['weightKg'] = $weightKg;

        return $self;
    }
}
