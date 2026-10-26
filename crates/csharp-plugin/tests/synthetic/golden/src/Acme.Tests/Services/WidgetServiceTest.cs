using System.Threading.Tasks;

namespace Acme.Tests.Services;

public class WidgetServiceTest : TestBase
{
    [Fact]
    public async Task Create_Works()
    {
        var widget = await this.client.Widgets.Create(
new() { Name = "name" },
TestContext.Current.CancellationToken
);
        widget.Validate();
    }

    [Fact]
    public async Task Retrieve_Works()
    {
        var widget = await this.client.Widgets.Retrieve(
"widget_id",
new(),
TestContext.Current.CancellationToken
);
        widget.Validate();
    }

    [Fact]
    public async Task List_Works()
    {
        var widget = await this.client.Widgets.List(
new(),
TestContext.Current.CancellationToken
);
        widget.Validate();
    }

    [Fact]
    public async Task Delete_Works()
    {
        await this.client.Widgets.Delete(
"widget_id",
new(),
TestContext.Current.CancellationToken
);
    }
}
