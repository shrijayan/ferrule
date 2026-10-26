using System.Threading.Tasks;

namespace Acme.Tests.Services.Widgets;

public class PartServiceTest : TestBase
{
    [Fact]
    public async Task Retrieve_Works()
    {
        var part = await this.client.Widgets.Parts.Retrieve(
0,
new() { WidgetID = "widget_id" },
TestContext.Current.CancellationToken
);
        part.Validate();
    }
}
