<?php

declare(strict_types=1);

namespace Acme\Services\Widgets;

use Acme\Client;
use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\Core\Util;
use Acme\RequestOptions;
use Acme\ServiceContracts\Widgets\PartsContract;
use Acme\Widgets\Parts\PartRetrieveResponse;

/**
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
final class PartsService implements PartsContract
{
    /**
     * @api
     */
    public PartsRawService $raw;

    /**
     * @internal
     */
    public function __construct(private Client $client)
    {
        $this->raw = new PartsRawService($client);
    }

    /**
     * @api
     *
     * Retrieve one part of a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function retrieve(
        int $partNumber,
        string $widgetID,
        RequestOptions|array|null $requestOptions = null,
    ): PartRetrieveResponse {
        $params = Util::removeNulls(['widgetID' => $widgetID]);

        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->retrieve($partNumber, params: $params, requestOptions: $requestOptions);

        return $response->parse();
    }
}
