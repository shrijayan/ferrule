<?php

declare(strict_types=1);

namespace Acme\ServiceContracts\Widgets;

use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\Widgets\Parts\PartRetrieveParams;
use Acme\Widgets\Parts\PartRetrieveResponse;

/**
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
interface PartsRawContract
{
    /**
     * @api
     *
     * @param array<string,mixed>|PartRetrieveParams $params
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<PartRetrieveResponse>
     *
     * @throws APIException
     */
    public function retrieve(
        int $partNumber,
        array|PartRetrieveParams $params,
        RequestOptions|array|null $requestOptions = null,
    ): BaseResponse;
}
