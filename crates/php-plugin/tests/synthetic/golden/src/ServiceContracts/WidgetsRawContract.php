<?php

declare(strict_types=1);

namespace Acme\ServiceContracts;

use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\Widgets\Widget;
use Acme\Widgets\WidgetCreateParams;
use Acme\Widgets\WidgetListParams;
use Acme\Widgets\WidgetListResponse;
use Acme\Widgets\WidgetStateResponse\Blocked;
use Acme\Widgets\WidgetStateResponse\Ready;

/**
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
interface WidgetsRawContract
{
    /**
     * @api
     *
     * @param array<string,mixed>|WidgetCreateParams $params
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Widget>
     *
     * @throws APIException
     */
    public function create(
        array|WidgetCreateParams $params,
        RequestOptions|array|null $requestOptions = null,
    ): BaseResponse;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Widget>
     *
     * @throws APIException
     */
    public function retrieve(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse;

    /**
     * @api
     *
     * @param array<string,mixed>|WidgetListParams $params
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<WidgetListResponse>
     *
     * @throws APIException
     */
    public function list(
        array|WidgetListParams $params,
        RequestOptions|array|null $requestOptions = null,
    ): BaseResponse;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<mixed>
     *
     * @throws APIException
     */
    public function delete(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @return BaseResponse<Ready|Blocked>
     *
     * @throws APIException
     */
    public function state(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): BaseResponse;
}
