<?php

namespace Tests\Services;

use Acme\Client;
use Acme\Core\Util;
use Acme\Widgets\Widget;
use Acme\Widgets\WidgetListResponse;
use PHPUnit\Framework\Attributes\CoversNothing;
use PHPUnit\Framework\Attributes\Test;
use PHPUnit\Framework\TestCase;

/**
 * @internal
 */
#[CoversNothing]
final class WidgetsTest extends TestCase
{
    protected Client $client;

    protected function setUp(): void
    {
        parent::setUp();

        $testUrl = Util::getenv('TEST_API_BASE_URL') ?: 'http://127.0.0.1:4010';
        $client = new Client(
            apiKey: 'My API Key',
            baseUrl: $testUrl,
        );

        $this->client = $client;
    }

    #[Test]
    public function testCreate(): void
    {
        $result = $this->client->widgets->create(name: 'name');

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertInstanceOf(Widget::class, $result);
    }

    #[Test]
    public function testCreateWithOptionalParams(): void
    {
        $result = $this->client->widgets->create(
            name: 'name',
            color: 'red',
            dimensions: ['width' => 0, 'height' => 0],
            tags: ['tags'],
        );

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertInstanceOf(Widget::class, $result);
    }

    #[Test]
    public function testRetrieve(): void
    {
        $result = $this->client->widgets->retrieve('widget_id');

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertInstanceOf(Widget::class, $result);
    }

    #[Test]
    public function testList(): void
    {
        $result = $this->client->widgets->list();

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertInstanceOf(WidgetListResponse::class, $result);
    }

    #[Test]
    public function testDelete(): void
    {
        $result = $this->client->widgets->delete('widget_id');

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertNull($result);
    }

    #[Test]
    public function testState(): void
    {
        $result = $this->client->widgets->state('widget_id');

        // @phpstan-ignore-next-line method.alreadyNarrowedType
        $this->assertNotNull($result);
    }
}
