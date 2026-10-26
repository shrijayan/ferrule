<?php

declare(strict_types=1);

namespace Acme\Services;

use Acme\Client;
use Acme\Core\Contracts\BaseResponse;
use Acme\Core\Exceptions\APIException;
use Acme\Core\Util;
use Acme\RequestOptions;
use Acme\ServiceContracts\WidgetsContract;
use Acme\Services\Widgets\PartsService;
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
final class WidgetsService implements WidgetsContract
{
    /**
     * @api
     */
    public WidgetsRawService $raw;

    /**
     * @api
     */
    public PartsService $parts;

    /**
     * @internal
     */
    public function __construct(private Client $client)
    {
        $this->raw = new WidgetsRawService($client);
        $this->parts = new PartsService($client);
    }

    /**
     * @api
     *
     * Creates a widget with the given name and parts. The SKU is assigned by the server.
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
    ): Widget {
        $params = Util::removeNulls(
            [
                'name' => $name,
                'color' => $color,
                'dimensions' => $dimensions,
                'tags' => $tags,
            ],
        );

        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->create(params: $params, requestOptions: $requestOptions);

        return $response->parse();
    }

    /**
     * @api
     *
     * Retrieve a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function retrieve(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): Widget {
        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->retrieve($widgetID, requestOptions: $requestOptions);

        return $response->parse();
    }

    /**
     * @api
     *
     * List widgets
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
    ): WidgetListResponse {
        $params = Util::removeNulls(
            ['colors' => $colors, 'createdAfter' => $createdAfter, 'limit' => $limit]
        );

        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->list(params: $params, requestOptions: $requestOptions);

        return $response->parse();
    }

    /**
     * @api
     *
     * Delete a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function delete(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): mixed {
        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->delete($widgetID, requestOptions: $requestOptions);

        return $response->parse();
    }

    /**
     * @api
     *
     * The state of a widget
     *
     * @param RequestOpts|null $requestOptions
     *
     * @throws APIException
     */
    public function state(
        string $widgetID, RequestOptions|array|null $requestOptions = null
    ): Ready|Blocked {
        // @phpstan-ignore-next-line argument.type
        $response = $this->raw->state($widgetID, requestOptions: $requestOptions);

        return $response->parse();
    }
}
