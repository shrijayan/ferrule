<?php

declare(strict_types=1);

namespace Acme\Services\Widgets;

use Acme\Client;
use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\ServiceContracts\Widgets\PartsRawContract;
use Acme\Widgets\Parts\PartRetrieveParams;
use Acme\Widgets\Parts\PartRetrieveResponse;

/**
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
final class PartsRawService implements PartsRawContract
{
    // @phpstan-ignore-next-line
    /**
     * @internal
     */
    public function __construct(private Client $client) {}

    /**
     * @api
     *
     * Retrieve one part of a widget
     *
     * @param array{widgetID: string}|PartRetrieveParams $params
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
    ): BaseResponse {
        [$parsed, $options] = PartRetrieveParams::parseRequest(
            $params,
            $requestOptions,
        );
        $widgetID = $parsed['widgetID'];
        unset($parsed['widgetID']);

        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'get',
            path: ['v1/widgets/%1$s/parts/%2$s', $widgetID, $partNumber],
            options: $options,
            convert: PartRetrieveResponse::class,
        );
    }
}
