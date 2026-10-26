<?php

declare(strict_types=1);

namespace Acme\Widgets\Widget;

/**
 * Where the widget is in its life cycle.
 */
enum Status: string
{
    case DRAFT = 'draft';

    case ACTIVE = 'active';

    case RETIRED = 'retired';
}
