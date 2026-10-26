<?php

declare(strict_types=1);

namespace Acme\Services;

use Acme\Client;
use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\ServiceContracts\WidgetsRawContract;
use Acme\Widgets\Widget;
use Acme\Widgets\WidgetCreateParams\Color;
use Acme\Widgets\WidgetCreateParams\Dimensions;
use Acme\Widgets\WidgetCreateParams;
use Acme\Widgets\WidgetListParams;
use Acme\Widgets\WidgetListResponse;
use Acme\Widgets\WidgetStateResponse\Blocked;
use Acme\Widgets\WidgetStateResponse\Ready;
use Acme\Widgets\WidgetStateResponse;

/**
 * @phpstan-import-type DimensionsShape from \Acme\Widgets\WidgetCreateParams\Dimensions
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
final class WidgetsRawService implements WidgetsRawContract
{
    // @phpstan-ignore-next-line
    /**
     * @internal
     */
    public function __construct(private Client $client) {}

    /**
     * @api
     *
     * Creates a widget with the given name and parts. The SKU is assigned by the server.
     *
     * @param array{
     *   name: string,
     *   color?: Color|value-of<Color>,
     *   dimensions?: Dimensions|DimensionsShape,
     *   tags?: list<string>,
     * }|WidgetCreateParams $params
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Widget>
     *
     * @throws APIException
     */
    public function create(
        array|WidgetCreateParams $params,
        RequestOptions|array|null $requestOptions = null,
    ): BaseResponse {
        [$parsed, $options] = WidgetCreateParams::parseRequest(
            $params,
            $requestOptions,
        );

        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'post',
            path: 'v1/widgets',
            body: (object) $parsed,
            options: $options,
            convert: Widget::class,
        );
    }

    /**
     * @api
     *
     * Retrieve a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Widget>
     *
     * @throws APIException
     */
    public function retrieve(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse {
        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'get',
            path: ['v1/widgets/%1$s', $widgetID],
            options: $requestOptions,
            convert: Widget::class,
        );
    }

    /**
     * @api
     *
     * List widgets
     *
     * @param array{
     *   colors?: list<string>, createdAfter?: \DateTimeInterface, limit?: int
     * }|WidgetListParams $params
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<WidgetListResponse>
     *
     * @throws APIException
     */
    public function list(
        array|WidgetListParams $params,
        RequestOptions|array|null $requestOptions = null,
    ): BaseResponse {
        [$parsed, $options] = WidgetListParams::parseRequest(
            $params,
            $requestOptions,
        );

        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'get',
            path: 'v1/widgets',
            query: $parsed,
            options: $options,
            convert: WidgetListResponse::class,
        );
    }

    /**
     * @api
     *
     * Delete a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<mixed>
     *
     * @throws APIException
     */
    public function delete(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse {
        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'delete',
            path: ['v1/widgets/%1$s', $widgetID],
            options: $requestOptions,
            convert: null,
        );
    }

    /**
     * @api
     *
     * The state of a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Ready|Blocked>
     *
     * @throws APIException
     */
    public function state(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse {
        // @phpstan-ignore-next-line return.type
        return $this->client->request(
            method: 'get',
            path: ['v1/widgets/%1$s/state', $widgetID],
            options: $requestOptions,
            convert: WidgetStateResponse::class,
        );
    }
}
