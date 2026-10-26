using System.Collections.Frozen;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;

namespace Acme.Models.Widgets.Parts;

[JsonConverter(typeof(JsonModelConverter<PartRetrieveResponse, PartRetrieveResponseFromRaw>))]
public sealed record class PartRetrieveResponse : JsonModel
{
    public required string ID
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<string>("id");
        }
init { this._rawData.Set("id", value); }
    }

    /// <summary>
    /// The part's stock keeping unit.
    /// </summary>
    public required string SKU
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNotNullClass<string>("sku");
        }
init { this._rawData.Set("sku", value); }
    }

    public ErrorInfo? Error
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<ErrorInfo>("error");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("error", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.ID;
_ = this.SKU;
this.Error?.Validate();
    }

    public PartRetrieveResponse() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public PartRetrieveResponse(PartRetrieveResponse partRetrieveResponse)
        : base(partRetrieveResponse) { }
#pragma warning restore CS8618

    public PartRetrieveResponse(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    PartRetrieveResponse(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="PartRetrieveResponseFromRaw.FromRawUnchecked"/>
    public static PartRetrieveResponse FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }
}

class PartRetrieveResponseFromRaw : IFromRawJson<PartRetrieveResponse>
{
    /// <inheritdoc/>
    public PartRetrieveResponse FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        PartRetrieveResponse.FromRawUnchecked(rawData);
}
