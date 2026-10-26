<?php

declare(strict_types=1);

namespace Acme\ServiceContracts;

use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\RequestOptions;
use Acme\Widgets\Widget;
use Acme\Widgets\WidgetCreateParams\Color;
use Acme\Widgets\WidgetCreateParams\Dimensions;
use Acme\Widgets\WidgetListResponse;
use Acme\Widgets\WidgetStateResponse\Blocked;
use Acme\Widgets\WidgetStateResponse\Ready;

/**
 * @phpstan-import-type DimensionsShape from \Acme\Widgets\WidgetCreateParams\Dimensions
 * @phpstan-import-type RequestOpts from \Acme\RequestOptions
 */
interface WidgetsContract
{
    /**
     * @api
     *
     * @param string $name The widget's display name.
     * @param Color|value-of<Color> $color
     * @param Dimensions|DimensionsShape $dimensions
     * @param list<string> $tags
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function create(
        string $name,
        Color|string|null $color = null,
        Dimensions|array|null $dimensions = null,
        ?array $tags = null,
        RequestOptions|array|null $requestOptions = null,
    ): Widget;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function retrieve(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): Widget;

    /**
     * @api
     *
     * @param list<string> $colors
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function list(
        ?array $colors = null,
        ?\DateTimeInterface $createdAfter = null,
        ?int $limit = null,
        RequestOptions|array|null $requestOptions = null,
    ): WidgetListResponse;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function delete(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): mixed;

    /**
     * @api
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function state(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): Ready|Blocked;
}
