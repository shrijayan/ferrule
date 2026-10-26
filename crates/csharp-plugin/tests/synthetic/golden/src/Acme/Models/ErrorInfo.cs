using System.Collections.Frozen;
using System.Collections.Generic;
using System.Diagnostics.CodeAnalysis;
using System.Text.Json;
using System.Text.Json.Serialization;
using Acme.Core;

namespace Acme.Models;

[JsonConverter(typeof(JsonModelConverter<ErrorInfo, ErrorInfoFromRaw>))]
public sealed record class ErrorInfo : JsonModel
{
    public string? Code
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("code");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("code", value);
}
    }

    public string? Message
    {
        get
        {
            this._rawData.Freeze();
            return this._rawData.GetNullableClass<string>("message");
        }
init
{
if (value == null)
{
return;
}

this._rawData.Set("message", value);
}
    }

    /// <inheritdoc/>
    public override void Validate()
    {
_ = this.Code;
_ = this.Message;
    }

    public ErrorInfo() { }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    public ErrorInfo(ErrorInfo errorInfo)
        : base(errorInfo) { }
#pragma warning restore CS8618

    public ErrorInfo(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }

#pragma warning disable CS8618
    [SetsRequiredMembers]
    ErrorInfo(FrozenDictionary<string, JsonElement> rawData)
    {
        this._rawData = new(rawData);
    }
#pragma warning restore CS8618

    /// <inheritdoc cref="ErrorInfoFromRaw.FromRawUnchecked"/>
    public static ErrorInfo FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData)
    {
        return new(FrozenDictionary.ToFrozenDictionary(rawData));
    }
}

class ErrorInfoFromRaw : IFromRawJson<ErrorInfo>
{
    /// <inheritdoc/>
    public ErrorInfo FromRawUnchecked(IReadOnlyDictionary<string, JsonElement> rawData) =>
        ErrorInfo.FromRawUnchecked(rawData);
}
