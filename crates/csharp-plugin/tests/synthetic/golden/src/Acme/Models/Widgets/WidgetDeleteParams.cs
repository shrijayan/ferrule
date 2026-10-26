using System;
using System.Collections.Frozen;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Net.Http;
using System.Text.Json;
using Acme.Core;

namespace Acme.Models.Widgets;

/// <summary>
/// Delete a widget
///
/// <para>NOTE: Do not inherit from this type outside the SDK unless you're okay with
/// breaking changes in non-major versions. We may add new methods in the future that
/// cause existing derived classes to break.</para>
/// </summary>
public record class WidgetDeleteParams : ParamsBase
{
    public string? WidgetID { get; init; }

    public WidgetDeleteParams() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public WidgetDeleteParams(WidgetDeleteParams widgetDeleteParams)
        : base(widgetDeleteParams)
    {
this.WidgetID = widgetDeleteParams.WidgetID;
    }
#pragma warning restore CS8618

    public WidgetDeleteParams(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    WidgetDeleteParams(
FrozenDictionary<string, JsonElement> rawHeaderData,
FrozenDictionary<string, JsonElement> rawQueryData,
string widgetID
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
this.WidgetID = widgetID;
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="IFromRawJson{T}.FromRawUnchecked"/>
    public static WidgetDeleteParams FromRawUnchecked(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData,
string widgetID
    )
    {
        return new(
FrozenDictionary.ToFrozenDictionary(rawHeaderData),
FrozenDictionary.ToFrozenDictionary(rawQueryData),
widgetID
        );
    }

    public override string ToString() =>
        JsonSerializer.Serialize(
            FriendlyJsonPrinter.PrintValue(
                new Dictionary<string, JsonElement>()
                {
["WidgetID"] = JsonSerializer.SerializeToElement(this.WidgetID),
["HeaderData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawHeaderData.Freeze())),
["QueryData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawQueryData.Freeze())),
                }
            ),
            ModelBase.ToStringSerializerOptions
        );

    public virtual bool Equals(WidgetDeleteParams? other)
    {
        if (other == null)
        {
            return false;
        }
        return (this.WidgetID?.Equals(other.WidgetID) ?? other.WidgetID == null)
            && this._rawHeaderData.Equals(other._rawHeaderData)
            && this._rawQueryData.Equals(other._rawQueryData);
    }

    public override Uri Url(ClientOptions options)
    {
        return new UriBuilder(options.BaseUrl.ToString().TrimEnd('/') + string.Format("/v1/widgets/{0}", this.WidgetID))
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
