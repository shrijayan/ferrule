using System;
using System.Collections.Frozen;
using System.Collections.Generic;
using System.Collections.Immutable;
using System.Diagnostics.CodeAnalysis;
using System.Net.Http;
using System.Text.Json;
using Acme.Core;

namespace Acme.Models.Widgets;

/// <summary>
/// List widgets
///
/// <para>NOTE: Do not inherit from this type outside the SDK unless you're okay with
/// breaking changes in non-major versions. We may add new methods in the future that
/// cause existing derived classes to break.</para>
/// </summary>
public record class WidgetListParams : ParamsBase
{
    public IReadOnlyList<string>? Colors
    {
        get
        {
            this._rawQueryData.Freeze();
            return this._rawQueryData.GetNullableStruct<ImmutableArray<string>>("colors");
        }
init
{
if (value == null)
{
return;
}

this._rawQueryData.Set<ImmutableArray<string>?>("colors", value == null ? null : ImmutableArray.ToImmutableArray(value));
}
    }

    public DateTimeOffset? CreatedAfter
    {
        get
        {
            this._rawQueryData.Freeze();
            return this._rawQueryData.GetNullableStruct<DateTimeOffset>("created_after");
        }
init
{
if (value == null)
{
return;
}

this._rawQueryData.Set("created_after", value);
}
    }

    public long? Limit
    {
        get
        {
            this._rawQueryData.Freeze();
            return this._rawQueryData.GetNullableStruct<long>("limit");
        }
init
{
if (value == null)
{
return;
}

this._rawQueryData.Set("limit", value);
}
    }

    public WidgetListParams() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public WidgetListParams(WidgetListParams widgetListParams)
        : base(widgetListParams) { }
#pragma warning restore CS8618

    public WidgetListParams(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    WidgetListParams(
FrozenDictionary<string, JsonElement> rawHeaderData,
FrozenDictionary<string, JsonElement> rawQueryData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="IFromRawJson{T}.FromRawUnchecked"/>
    public static WidgetListParams FromRawUnchecked(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData
    )
    {
        return new(
FrozenDictionary.ToFrozenDictionary(rawHeaderData),
FrozenDictionary.ToFrozenDictionary(rawQueryData)
        );
    }

    public override string ToString() =>
        JsonSerializer.Serialize(
            FriendlyJsonPrinter.PrintValue(
                new Dictionary<string, JsonElement>()
                {
["HeaderData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawHeaderData.Freeze())),
["QueryData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawQueryData.Freeze())),
                }
            ),
            ModelBase.ToStringSerializerOptions
        );

    public virtual bool Equals(WidgetListParams? other)
    {
        if (other == null)
        {
            return false;
        }
        return this._rawHeaderData.Equals(other._rawHeaderData)
            && this._rawQueryData.Equals(other._rawQueryData);
    }

    public override Uri Url(ClientOptions options)
    {
        return new UriBuilder(options.BaseUrl.ToString().TrimEnd('/') + "/v1/widgets")
        {
            Query = this.QueryString(options),
        }.Uri;
    }

    internal override void AddHeadersToRequest(HttpRequestMessage request, ClientOptions options)
    {
        ParamsBase.AddDefaultHeaders(request, options);
        foreach (var item in this.RawHeaderData)
        {
            ParamsBase.AddHeaderElementToRequest(request, item.Key, item.Value);
        }
    }

    public override int GetHashCode()
    {
        return 0;
    }
}
