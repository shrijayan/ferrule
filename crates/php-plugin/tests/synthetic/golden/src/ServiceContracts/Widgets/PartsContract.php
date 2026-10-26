<?php

declare(strict_types=1);

namespace Acme\ServiceContracts\Widgets;

use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\Widgets\Parts\PartRetrieveResponse;

/**
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
interface PartsContract
{
    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function retrieve(
        int $partNumber,
        string $widgetID,
        RequestOptions|array|null $requestOptions = null,
    ): PartRetrieveResponse;
}
