using System;
using System.Collections.Frozen;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Net.Http;
using System.Text.Json;
using Acme.Core;

namespace Acme.Models.Widgets.Parts;

/// <summary>
/// Retrieve one part of a widget
///
/// <para>NOTE: Do not inherit from this type outside the SDK unless you're okay with
/// breaking changes in non-major versions. We may add new methods in the future that
/// cause existing derived classes to break.</para>
/// </summary>
public record class PartRetrieveParams : ParamsBase
{
    public required string WidgetID { get; init; }

    public long? PartNumber { get; init; }

    public PartRetrieveParams() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public PartRetrieveParams(PartRetrieveParams partRetrieveParams)
        : base(partRetrieveParams)
    {
this.WidgetID = partRetrieveParams.WidgetID;
this.PartNumber = partRetrieveParams.PartNumber;
    }
#pragma warning restore CS8618

    public PartRetrieveParams(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    PartRetrieveParams(
FrozenDictionary<string, JsonElement> rawHeaderData,
FrozenDictionary<string, JsonElement> rawQueryData,
string widgetID,
long partNumber
    )
    {
this._rawHeaderData = new(rawHeaderData);
this._rawQueryData = new(rawQueryData);
this.WidgetID = widgetID;
this.PartNumber = partNumber;
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="IFromRawJson{T}.FromRawUnchecked"/>
    public static PartRetrieveParams FromRawUnchecked(
IReadOnlyDictionary<string, JsonElement> rawHeaderData,
IReadOnlyDictionary<string, JsonElement> rawQueryData,
string widgetID,
long partNumber
    )
    {
        return new(
FrozenDictionary.ToFrozenDictionary(rawHeaderData),
FrozenDictionary.ToFrozenDictionary(rawQueryData),
widgetID,
partNumber
        );
    }

    public override string ToString() =>
        JsonSerializer.Serialize(
            FriendlyJsonPrinter.PrintValue(
                new Dictionary<string, JsonElement>()
                {
["WidgetID"] = JsonSerializer.SerializeToElement(this.WidgetID),
["PartNumber"] = JsonSerializer.SerializeToElement(this.PartNumber),
["HeaderData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawHeaderData.Freeze())),
["QueryData"] = FriendlyJsonPrinter.PrintValue(JsonSerializer.SerializeToElement(this._rawQueryData.Freeze())),
                }
            ),
            ModelBase.ToStringSerializerOptions
        );

    public virtual bool Equals(PartRetrieveParams? other)
    {
        if (other == null)
        {
            return false;
        }
        return this.WidgetID.Equals(other.WidgetID)
            && (this.PartNumber?.Equals(other.PartNumber) ?? other.PartNumber == null)
            && this._rawHeaderData.Equals(other._rawHeaderData)
            && this._rawQueryData.Equals(other._rawQueryData);
    }

    public override Uri Url(ClientOptions options)
    {
        return new UriBuilder(options.BaseUrl.ToString().TrimEnd('/') + string.Format("/v1/widgets/{0}/parts/{1}", this.WidgetID, this.PartNumber))
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
