using System.Collections.Frozen;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;

namespace Acme.Models.Widgets;

[JsonConverter(typeof(JsonModelConverter<WidgetListResponse, WidgetListResponseFromRaw>))]
public sealed record class WidgetListResponse : JsonModel
{
    public required IReadOnlyList<Widget> Data
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullStruct<ImmutableArray<Widget>>("data");
        }
init { this._rawData.Set<ImmutableArray<Widget>>("data", ImmutableArray.ToImmutableArray(value)); }
    }

    public string? NextCursor
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("next_cursor");
        }
init { this._rawData.Set("next_cursor", value); }
    }

    /// <inheritdoc/>
    public override void Validate()
    {
foreach (var item in this.Data)
{
item.Validate();
}
_ = this.NextCursor;
    }

    public WidgetListResponse() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public WidgetListResponse(WidgetListResponse widgetListResponse)
        : base(widgetListResponse) { }
#pragma warning restore CS8618

    public WidgetListResponse(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    WidgetListResponse(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="WidgetListResponseFromRaw.FromRawUnchecked"/>
    public static WidgetListResponse FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }

    [SetsRequiredMembers]
    public WidgetListResponse(IReadOnlyList<Widget> data)
        : this()
    {
        this.Data = data;
    }
}

class WidgetListResponseFromRaw : IFromRawJson<WidgetListResponse>
{
    /// <inheritdoc/>
    public WidgetListResponse FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        WidgetListResponse.FromRawUnchecked(rawData);
}
